//! A Task Execution Service (TES) backend.
//!
//! Learn more about the TES API specification [here][tes].
//!
//! [tes]: https://www.ga4gh.org/product/task-execution-service-tes/

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
#[cfg(windows)]
use std::os::windows::process::ExitStatusExt;
use std::process::ExitStatus;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use anyhow::anyhow;
use async_trait::async_trait;
use crankshaft_config::backend::tes::Config;
use crankshaft_config::backend::tes::http::HttpAuthConfig;
use crankshaft_events::Event;
use crankshaft_events::TaskId;
use crankshaft_events::next_task_id;
use crankshaft_events::send_event;
use futures::FutureExt as _;
use futures::future::BoxFuture;
use nonempty::NonEmpty;
use tes::auth::BasicAuthorizer;
use tes::auth::BearerTokenAuthorizer;
use tes::v1::Client;
use tes::v1::client::strategy::ExponentialFactorBackoff;
use tes::v1::types::requests::GetTaskParams;
use tes::v1::types::requests::View;
use tes::v1::types::task::State as TesState;
use tokio::select;
use tokio::sync::Semaphore;
use tokio::sync::broadcast;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use tracing::error;
use tracing::info;

use super::TaskRunError;
use crate::Task;
use crate::service::name::GeneratorIterator;
use crate::service::name::UniqueAlphanumeric;
use crate::service::runner::backend::tes::monitor::TaskMonitor;
use crate::task::ExecutionResult;

mod monitor;

/// The default poll interval for querying task status.
const DEFAULT_INTERVAL: Duration = Duration::from_secs(1);

/// The maximum delay between retry attempts.
const MAX_RETRY_DELAY: Duration = Duration::from_secs(30);

/// The default maximum number of concurrent requests the backend will make.
const DEFAULT_MAX_CONCURRENT_REQUESTS: usize = 10;

/// Shared state between tasks.
#[derive(Debug)]
struct BackendState {
    /// The TES client.
    client: Client,
    /// The poll interval for checking on task status.
    interval: Duration,
    /// The number of retries to attempt.
    retries: usize,
    /// The retry policy to use for client operations.
    policy: ExponentialFactorBackoff,
    /// The permits for ensuring a maximum number of concurrent server requests.
    permits: Semaphore,
    /// Whether to read task resource usage from the server's task log
    /// metadata.
    resource_usage_metadata: bool,
}

impl BackendState {
    /// Gets the retry policy for the backend.
    fn policy(&self) -> impl Iterator<Item = Duration> + use<'_> {
        self.policy.clone().take(self.retries)
    }
}

/// A backend driven by the Task Execution Service (TES) schema.
#[derive(Debug)]
pub struct Backend {
    /// The unique name generator for tasks without names.
    names: Arc<Mutex<GeneratorIterator<UniqueAlphanumeric>>>,
    /// The backend state shared between tasks.
    state: Arc<BackendState>,
    /// The TES task monitor.
    monitor: TaskMonitor,
}

impl Backend {
    /// Creates a new TES [`Backend`].
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use std::sync::Mutex;
    ///
    /// use crankshaft_config::backend::tes::Config;
    /// use crankshaft_engine::service::name::GeneratorIterator;
    /// use crankshaft_engine::service::name::UniqueAlphanumeric;
    /// use crankshaft_engine::service::runner::backend::tes::Backend;
    /// use url::Url;
    ///
    /// let url = "http://localhost:8000".parse::<Url>()?;
    /// let config = Config::builder().url(url).build();
    ///
    /// let names = Arc::new(Mutex::new(GeneratorIterator::new(
    ///     UniqueAlphanumeric::default_with_expected_generations(4096),
    ///     4096,
    /// )));
    ///
    /// # tokio_test::block_on(async {
    /// let backend = Backend::initialize(config, names).await;
    /// # });
    ///
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub async fn initialize(
        config: Config,
        names: Arc<Mutex<GeneratorIterator<UniqueAlphanumeric>>>,
    ) -> Self {
        let client = Client::builder().url(config.url().clone());

        let client = match &config.http().auth {
            Some(HttpAuthConfig::Basic { username, password }) => {
                client.authorizer(BasicAuthorizer::new(username, password.clone()))
            }
            Some(HttpAuthConfig::Bearer { token }) => {
                client.authorizer(BearerTokenAuthorizer::new(token))
            }
            None => client,
        };

        // SAFETY: the only required field of `builder` is the `url`, which we
        // provided earlier.
        Self::initialize_with_client(config, client.try_build().unwrap(), names).await
    }

    /// Creates a new TES [`Backend`] with the given TES client.
    ///
    /// The given client's endpoint and authorizer are used instead of
    /// `config.url` and `config.http.auth`. Other HTTP settings are still
    /// read from `config`.
    pub async fn initialize_with_client(
        config: Config,
        client: Client,
        names: Arc<Mutex<GeneratorIterator<UniqueAlphanumeric>>>,
    ) -> Self {
        let resource_usage_metadata = config.resource_usage_metadata();
        let (_, http, interval) = config.into_parts();

        let state = Arc::new(BackendState {
            client,
            interval: interval
                .map(Duration::from_secs)
                .unwrap_or(DEFAULT_INTERVAL),
            retries: http.retries.unwrap_or_default() as usize,
            policy: ExponentialFactorBackoff::from_millis(1000, 2.0).max_delay(MAX_RETRY_DELAY),
            permits: Semaphore::new(
                http.max_concurrency
                    .unwrap_or(DEFAULT_MAX_CONCURRENT_REQUESTS),
            ),
            resource_usage_metadata,
        });

        // SAFETY: the name generator should _never_ run out of entries.
        let monitor_name = names.lock().unwrap().next().unwrap();
        let monitor = TaskMonitor::new(monitor_name, state.clone()).await;

        Self {
            names,
            state,
            monitor,
        }
    }

    /// Attaches to a TES task created by an earlier `run`, possibly by another
    /// process.
    ///
    /// The returned future behaves like the one from `run`.
    pub fn attach(
        &self,
        tes_id: String,
        name: String,
        events: Option<broadcast::Sender<Event>>,
        token: CancellationToken,
    ) -> BoxFuture<'static, Result<NonEmpty<ExecutionResult>, TaskRunError>> {
        let task_id = next_task_id();
        let monitor = self.monitor.clone();
        let state = self.state.clone();

        async move {
            // Read the task to find the group tag it was created with
            let permit = state
                .permits
                .acquire()
                .await
                .context("failed to acquire network request permit")?;

            let task = select! {
                // Always poll the cancellation token first
                biased;
                _ = token.cancelled() => {
                    return Err(TaskRunError::Canceled);
                }
                res = state.client.get_task(
                    &tes_id,
                    Some(&GetTaskParams { view: View::Basic }),
                    state.policy(),
                ) => {
                    res.context("failed to get task information from TES server")?
                }
            };

            // Drop the permit now that the request has completed
            drop(permit);

            let tag = task
                .into_task()
                .and_then(|t| t.tags)
                .and_then(|mut tags| tags.remove(monitor::CRANKSHAFT_GROUP_TAG_NAME))
                .with_context(|| format!("TES task `{tes_id}` was not created by Crankshaft"))?;

            info!("attaching to TES task `{tes_id}` (task `{name}`)");

            let (completed_tx, completed_rx) = oneshot::channel();
            monitor
                .attach_task(
                    task_id,
                    name.clone(),
                    tes_id.clone(),
                    tag,
                    events.clone(),
                    completed_tx,
                )
                .await;

            Self::track(
                state,
                monitor,
                task_id,
                name,
                tes_id,
                events,
                token,
                completed_rx,
            )
            .await
        }
        .boxed()
    }

    /// Tracks a TES task from creation until it finishes.
    ///
    /// The task must already be in the monitor with its TES ID.
    ///
    /// Sends the task's creation and completion events, removes the task from
    /// the monitor, and cancels the TES task if `token` is canceled.
    #[allow(clippy::too_many_arguments)]
    async fn track(
        state: Arc<BackendState>,
        monitor: TaskMonitor,
        task_id: TaskId,
        task_name: String,
        tes_id: String,
        events: Option<broadcast::Sender<Event>>,
        token: CancellationToken,
        completed: oneshot::Receiver<Result<()>>,
    ) -> Result<NonEmpty<ExecutionResult>, TaskRunError> {
        let task_token = CancellationToken::new();

        send_event!(
            events,
            Event::TaskCreated {
                id: task_id,
                name: task_name.clone(),
                tes_id: Some(tes_id.clone()),
                token: task_token.clone()
            }
        );

        let result = select! {
            // Always poll the cancellation token first
            biased;
            _ = task_token.cancelled() => {
                Err(TaskRunError::Canceled)
            }
            _ = token.cancelled() => {
                Err(TaskRunError::Canceled)
            }
            res = Self::wait_task(&state, &task_name, &tes_id, completed) => {
                res
            }
        };

        // Remove the task from the monitor
        monitor.remove_task(task_id).await;

        // Cancel the TES task if the task was canceled
        // At this point, log errors instead of returning a different one
        if let Err(TaskRunError::Canceled) = &result {
            match state.permits.acquire().await {
                Ok(permit) => {
                    info!("canceling TES task `{tes_id}` (task `{task_name}`)");

                    // Cancel the task
                    if let Err(e) = state.client.cancel_task(&tes_id, state.policy()).await {
                        error!("failed to cancel task with TES server: {e:#}");
                    }

                    // Drop the permit now that the request has completed
                    drop(permit);
                }
                Err(e) => {
                    error!("failed to acquire permit to cancel TES task: {e}");
                }
            }
        }

        // Send the completion event
        match &result {
            Ok(results) => send_event!(
                events,
                Event::TaskCompleted {
                    id: task_id,
                    exit_statuses: NonEmpty::collect(results.iter().map(|r| r.status)).unwrap(),
                }
            ),
            Err(TaskRunError::Canceled) => {
                send_event!(events, Event::TaskCanceled { id: task_id })
            }
            Err(TaskRunError::Preempted) => {
                send_event!(events, Event::TaskPreempted { id: task_id })
            }
            Err(TaskRunError::Other(e)) => send_event!(
                events,
                Event::TaskFailed {
                    id: task_id,
                    message: format!("{e:#}")
                }
            ),
        }

        result
    }

    /// Waits for a task to complete.
    async fn wait_task(
        state: &BackendState,
        task_name: &str,
        tes_id: &str,
        completed: oneshot::Receiver<Result<()>>,
    ) -> Result<NonEmpty<ExecutionResult>, TaskRunError> {
        info!("waiting for TES task `{tes_id}` (task `{task_name}`) to complete");

        // Wait for notification from the monitor that the task has completed
        completed
            .await
            .context("failed to wait for task completion")??;

        // Query for the state of the task
        let permit = state
            .permits
            .acquire()
            .await
            .context("failed to acquire network request permit")?;

        let task = state
            .client
            .get_task(
                tes_id,
                Some(&GetTaskParams { view: View::Full }),
                state.policy(),
            )
            .await
            .context("failed to get task information from TES server")?
            .into_task()
            .context("returned task is not a full view")?;

        // Drop the permit now that the request has completed
        drop(permit);

        let task_state = task.state.unwrap_or_default();
        match task_state {
            TesState::Unknown
            | TesState::Queued
            | TesState::Initializing
            | TesState::Running
            | TesState::Paused
            | TesState::Canceling => Err(TaskRunError::Other(anyhow!(
                "TES task is not in a completed state"
            ))),
            TesState::Complete | TesState::ExecutorError => {
                // Task completed or had an error
                if task_state == TesState::Complete {
                    info!("TES task `{tes_id}` (task `{task_name}`) has completed");
                } else {
                    info!("TES task `{tes_id}` (task `{task_name}`) has failed");
                }

                // There may be multiple task logs due to internal retries by
                // the TES server Therefore, we're only
                // interested in the last log
                let logs = task.logs.unwrap_or_default();
                let task_log = logs.last().context(
                    "invalid response from TES server: completed task is missing task logs",
                )?;

                // Iterate the exit code from each executor log
                Ok(
                    NonEmpty::collect(task_log.logs.iter().enumerate().map(|(idx, executor)| {
                        // See WEXITSTATUS from wait(2) to explain the shift
                        #[cfg(unix)]
                        let status = ExitStatus::from_raw(executor.exit_code << 8);

                        #[cfg(windows)]
                        let status = ExitStatus::from_raw(executor.exit_code as u32);

                        ExecutionResult {
                            // Realistically, the executor for any given log should always exist
                            image: task.executors.get(idx).map(|e| e.image.clone()),
                            status,
                        }
                    }))
                    .context(
                        "invalid response from TES server: completed task is missing executor logs",
                    )?,
                )
            }
            TesState::SystemError => {
                info!("TES task `{tes_id}` (task `{task_name}`) has failed with a system error");

                let messages = task
                    .logs
                    .unwrap_or_default()
                    .last()
                    .and_then(|l| l.system_logs.as_ref().map(|l| l.join("\n")))
                    .unwrap_or_default();

                Err(TaskRunError::Other(anyhow!(
                    "task failed due to system error:\n\n{messages}"
                )))
            }
            TesState::Canceled => {
                info!("TES task `{tes_id}` (task `{task_name}`) has been canceled");
                Err(TaskRunError::Canceled)
            }
            TesState::Preempted => {
                info!("TES task `{tes_id}` (task `{task_name}`) has been preempted");
                Err(TaskRunError::Preempted)
            }
        }
    }
}

#[async_trait]
impl crate::Backend for Backend {
    fn default_name(&self) -> &'static str {
        "tes"
    }

    /// Runs a task in a backend.
    fn run(
        &self,
        task: Task,
        events: Option<broadcast::Sender<Event>>,
        token: CancellationToken,
    ) -> Result<BoxFuture<'static, Result<NonEmpty<ExecutionResult>, TaskRunError>>> {
        let task_id = next_task_id();
        let names = self.names.clone();
        let monitor = self.monitor.clone();
        let state = self.state.clone();

        Ok(async move {
            // Generate a name of the task if one wasn't provided
            let task_name = task.name.clone().unwrap_or_else(|| {
                // SAFETY: the name generator should _never_ run out of entries.
                names.lock().unwrap().next().unwrap()
            });

            let mut task = tes::v1::types::requests::Task::try_from(task)?;

            // Add the task to the monitor
            let (completed_tx, completed_rx) = oneshot::channel();
            let tag = monitor
                .add_task(task_id, task_name.clone(), events.clone(), completed_tx)
                .await;

            // Create the TES task
            let created = async {
                task.tags
                    .get_or_insert_default()
                    .insert(monitor::CRANKSHAFT_GROUP_TAG_NAME.to_string(), tag);

                let permit = state
                    .permits
                    .acquire()
                    .await
                    .context("failed to acquire network request permit")?;

                let id = select! {
                    // Always poll the cancellation token first
                    biased;
                    _ = token.cancelled() => {
                        return Err(TaskRunError::Canceled);
                    }
                    res = state.client.create_task(&task, state.policy()) => {
                        res.context("failed to create task with TES server")?.id
                    }
                };

                // Drop the permit now that the request has completed
                drop(permit);
                Ok(id)
            }
            .await;

            let tes_id = match created {
                Ok(id) => id,
                Err(e) => {
                    monitor.remove_task(task_id).await;
                    return Err(e);
                }
            };

            // Associate the TES task id with the Crankshaft id in the monitor
            monitor.associate_task_id(task_id, tes_id.clone()).await;

            Self::track(
                state,
                monitor,
                task_id,
                task_name,
                tes_id,
                events,
                token,
                completed_rx,
            )
            .await
        }
        .boxed())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::collections::HashMap;

    use axum::Json;
    use axum::Router;
    use axum::extract::Path;
    use axum::extract::RawQuery;
    use axum::extract::State;
    use axum::routing::get;
    use tes::v1::types::responses::CreatedTask;
    use tes::v1::types::responses::ExecutorLog;
    use tes::v1::types::responses::ListTasks;
    use tes::v1::types::responses::Task as TesTask;
    use tes::v1::types::responses::TaskLog;
    use tes::v1::types::task::Executor;
    use url::Url;

    use super::*;
    use crate::task::Execution;

    /// The tasks held by a fake TES server, by TES ID.
    type Tasks = Arc<Mutex<HashMap<String, TesTask>>>;

    /// Starts a fake TES server and returns its URL.
    async fn serve(tasks: Tasks) -> Url {
        async fn list(
            State(tasks): State<Tasks>,
            RawQuery(query): RawQuery,
        ) -> Json<ListTasks<TesTask>> {
            let tag = url::form_urlencoded::parse(query.unwrap_or_default().as_bytes())
                .find(|(k, _)| k == "tag_value")
                .map(|(_, v)| v.into_owned());
            let tasks = tasks
                .lock()
                .unwrap()
                .values()
                .filter(|t| {
                    t.tags
                        .as_ref()
                        .and_then(|tags| tags.get(monitor::CRANKSHAFT_GROUP_TAG_NAME))
                        == tag.as_ref()
                })
                .cloned()
                .collect();
            Json(ListTasks {
                tasks,
                next_page_token: None,
            })
        }

        async fn create(
            State(tasks): State<Tasks>,
            Json(task): Json<tes::v1::types::requests::Task>,
        ) -> Json<CreatedTask> {
            let mut tasks = tasks.lock().unwrap();
            let id = format!("created-{}", tasks.len());
            tasks.insert(
                id.clone(),
                TesTask {
                    id: Some(id.clone()),
                    state: Some(TesState::Queued),
                    executors: task.executors,
                    tags: task.tags,
                    ..Default::default()
                },
            );
            Json(CreatedTask { id })
        }

        async fn get_one(State(tasks): State<Tasks>, Path(id): Path<String>) -> Json<TesTask> {
            Json(tasks.lock().unwrap()[&id].clone())
        }

        async fn cancel(
            State(tasks): State<Tasks>,
            Path(id): Path<String>,
        ) -> Json<serde_json::Value> {
            let id = id.strip_suffix(":cancel").unwrap();
            tasks.lock().unwrap().get_mut(id).unwrap().state = Some(TesState::Canceled);
            Json(serde_json::json!({}))
        }

        let app = Router::new()
            .route("/tasks", get(list).post(create))
            .route("/tasks/{id}", get(get_one).post(cancel))
            .with_state(tasks);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{addr}/").parse().unwrap()
    }

    /// Creates a backend that talks to the given server.
    async fn backend(url: Url) -> Backend {
        let names = Arc::new(Mutex::new(GeneratorIterator::new(
            UniqueAlphanumeric::default_with_expected_generations(16),
            16,
        )));
        Backend::initialize(Config::builder().url(url).interval(1).build(), names).await
    }

    /// Makes a TES task with the given group tag.
    fn tes_task(id: &str, state: TesState, tag: Option<&str>) -> TesTask {
        TesTask {
            id: Some(id.to_string()),
            state: Some(state),
            executors: vec![Executor {
                image: "ubuntu".to_string(),
                command: vec!["true".to_string()],
                ..Default::default()
            }],
            tags: tag.map(|t| {
                BTreeMap::from([(
                    monitor::CRANKSHAFT_GROUP_TAG_NAME.to_string(),
                    t.to_string(),
                )])
            }),
            ..Default::default()
        }
    }

    /// Marks a TES task complete with the given exit code.
    fn complete(task: &mut TesTask, exit_code: i32) {
        task.state = Some(TesState::Complete);
        task.logs = Some(vec![TaskLog {
            logs: vec![ExecutorLog {
                exit_code,
                ..Default::default()
            }],
            ..Default::default()
        }]);
    }

    /// Waits for an event that matches.
    async fn wait_for(rx: &mut broadcast::Receiver<Event>, f: impl Fn(&Event) -> bool) -> Event {
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let event = rx.recv().await.unwrap();
                if f(&event) {
                    return event;
                }
            }
        })
        .await
        .expect("timed out waiting for event")
    }

    #[tokio::test]
    async fn attach_waits_for_a_running_task() {
        let tasks = Tasks::default();
        tasks.lock().unwrap().insert(
            "t1".to_string(),
            tes_task("t1", TesState::Running, Some("old-tag")),
        );
        let backend = backend(serve(tasks.clone()).await).await;
        let (tx, mut rx) = broadcast::channel(16);

        let attached = tokio::spawn(backend.attach(
            "t1".to_string(),
            "task".to_string(),
            Some(tx),
            CancellationToken::new(),
        ));

        let created = wait_for(&mut rx, |e| matches!(e, Event::TaskCreated { .. })).await;
        assert!(matches!(
            created,
            Event::TaskCreated { name, tes_id: Some(tes_id), .. } if name == "task" && tes_id == "t1"
        ));
        wait_for(&mut rx, |e| matches!(e, Event::TaskStarted { .. })).await;

        complete(tasks.lock().unwrap().get_mut("t1").unwrap(), 3);
        let results = attached.await.unwrap().unwrap();
        assert_eq!(results.head.status.code(), Some(3));
        assert_eq!(results.head.image.as_deref(), Some("ubuntu"));
        wait_for(&mut rx, |e| matches!(e, Event::TaskCompleted { .. })).await;
    }

    #[tokio::test]
    async fn attach_returns_the_results_of_a_finished_task() {
        let tasks = Tasks::default();
        let mut task = tes_task("t1", TesState::Running, Some("old-tag"));
        complete(&mut task, 0);
        tasks.lock().unwrap().insert("t1".to_string(), task);
        let backend = backend(serve(tasks).await).await;

        let results = tokio::time::timeout(
            Duration::from_secs(10),
            backend.attach(
                "t1".to_string(),
                "task".to_string(),
                None,
                CancellationToken::new(),
            ),
        )
        .await
        .expect("attach should finish")
        .unwrap();
        assert_eq!(results.head.status.code(), Some(0));
    }

    #[tokio::test]
    async fn attach_rejects_a_task_without_a_crankshaft_tag() {
        let tasks = Tasks::default();
        tasks
            .lock()
            .unwrap()
            .insert("t1".to_string(), tes_task("t1", TesState::Running, None));
        let backend = backend(serve(tasks).await).await;

        let err = backend
            .attach(
                "t1".to_string(),
                "task".to_string(),
                None,
                CancellationToken::new(),
            )
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("not created by Crankshaft"),
            "{err}"
        );
    }

    #[tokio::test]
    async fn attach_cancels_the_tes_task_when_canceled() {
        let tasks = Tasks::default();
        tasks.lock().unwrap().insert(
            "t1".to_string(),
            tes_task("t1", TesState::Running, Some("old-tag")),
        );
        let backend = backend(serve(tasks.clone()).await).await;
        let (tx, mut rx) = broadcast::channel(16);
        let token = CancellationToken::new();

        let attached = tokio::spawn(backend.attach(
            "t1".to_string(),
            "task".to_string(),
            Some(tx),
            token.clone(),
        ));
        wait_for(&mut rx, |e| matches!(e, Event::TaskCreated { .. })).await;
        token.cancel();

        assert!(matches!(
            attached.await.unwrap(),
            Err(TaskRunError::Canceled)
        ));
        assert_eq!(tasks.lock().unwrap()["t1"].state, Some(TesState::Canceled));
        wait_for(&mut rx, |e| matches!(e, Event::TaskCanceled { .. })).await;
    }

    #[tokio::test]
    async fn run_uses_a_new_tag_while_only_attached_tasks_are_monitored() {
        let tasks = Tasks::default();
        tasks.lock().unwrap().insert(
            "t1".to_string(),
            tes_task("t1", TesState::Running, Some("old-tag")),
        );
        let backend = backend(serve(tasks.clone()).await).await;
        let (tx, mut rx) = broadcast::channel(16);
        let token = CancellationToken::new();

        let _attached = tokio::spawn(backend.attach(
            "t1".to_string(),
            "attached".to_string(),
            Some(tx.clone()),
            token.clone(),
        ));
        wait_for(
            &mut rx,
            |e| matches!(e, Event::TaskCreated { name, .. } if name == "attached"),
        )
        .await;

        let task = Task::builder()
            .name("new")
            .executions(NonEmpty::new(
                Execution::builder()
                    .images(["ubuntu"])
                    .unwrap()
                    .program("true")
                    .build(),
            ))
            .build();
        let _run =
            tokio::spawn(crate::Backend::run(&backend, task, Some(tx), token.clone()).unwrap());
        let created = wait_for(
            &mut rx,
            |e| matches!(e, Event::TaskCreated { name, .. } if name == "new"),
        )
        .await;
        let Event::TaskCreated {
            tes_id: Some(tes_id),
            ..
        } = created
        else {
            unreachable!()
        };

        let tag = tasks.lock().unwrap()[&tes_id]
            .tags
            .as_ref()
            .and_then(|t| t.get(monitor::CRANKSHAFT_GROUP_TAG_NAME).cloned())
            .unwrap();
        assert!(!tag.is_empty());
        assert_ne!(tag, "old-tag");
        token.cancel();
    }
}
