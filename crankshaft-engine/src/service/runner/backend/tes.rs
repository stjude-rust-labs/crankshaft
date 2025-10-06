//! A Task Execution Service (TES) backend.
//!
//! Learn more about the TES API specification [here][tes].
//!
//! [tes]: https://www.ga4gh.org/product/task-execution-service-tes/

use std::collections::HashMap;
use std::collections::HashSet;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
#[cfg(windows)]
use std::os::windows::process::ExitStatusExt;
use std::process::ExitStatus;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::time::SystemTime;

use anyhow::Context;
use anyhow::Result;
use anyhow::anyhow;
use async_trait::async_trait;
use crankshaft_config::backend::tes::Config;
use crankshaft_events::Event;
use crankshaft_events::next_task_id;
use crankshaft_events::send_event;
use futures::FutureExt as _;
use futures::future::BoxFuture;
use nonempty::NonEmpty;
use tes::v1::Client;
use tes::v1::client::strategy::ExponentialFactorBackoff;
use tes::v1::types::requests::GetTaskParams;
use tes::v1::types::requests::ListTasksParams;
use tes::v1::types::requests::MAX_PAGE_SIZE;
use tes::v1::types::requests::View;
use tes::v1::types::responses::ListTasks;
use tes::v1::types::task::State as TesState;
use tokio::select;
use tokio::sync::Semaphore;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::time::MissedTickBehavior;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::info;

use super::TaskRunError;
use crate::Task;
use crate::service::name::GeneratorIterator;
use crate::service::name::UniqueAlphanumeric;

/// The default poll interval for querying task status.
const DEFAULT_INTERVAL: Duration = Duration::from_secs(1);

/// The name of the tag used to group tasks together for monitoring.
const CRANKSHAFT_GROUP_TAG_NAME: &str = "crankshaft-task-group";

/// The maximum delay between retry attempts.
const MAX_RETRY_DELAY: Duration = Duration::from_secs(30);

/// The maximum number of messages the monitor channel will buffer before
/// blocking.
const MONITOR_CAPACITY: usize = 100;

/// The default maximum number of concurrent requests the backend will make.
const DEFAULT_MAX_CONCURRENT_REQUESTS: usize = 10;

/// Shared state between tasks.
#[derive(Debug)]
struct State {
    /// The TES client.
    client: Client,
    /// The poll interval for checking on task status.
    interval: Duration,
    /// The number of retries to attempt.
    retries: usize,
    /// The retry policy to use for client operations.
    policy: ExponentialFactorBackoff,
    /// The sender for the monitor using for monitoring new tasks.
    monitor: mpsc::Sender<MonitorRequest>,
    /// The permits for ensuring a maximum number of server requests.
    permits: Semaphore,
    /// The events sender for Crankshaft events.
    events: Option<broadcast::Sender<Event>>,
}

impl State {
    /// Gets the retry policy for the backend.
    fn policy(&self) -> impl Iterator<Item = Duration> + use<'_> {
        self.policy.clone().take(self.retries)
    }
}

/// A request to the task monitor.
enum MonitorRequest {
    /// Add a new task to the monitor.
    Add {
        /// The Crankshaft task id.
        id: u64,
        /// The sender for notifying the tag that should be used to create the
        /// TES task.
        tag: oneshot::Sender<String>,
        /// The sender for notifying the completion of the task.
        completed: oneshot::Sender<Result<()>>,
    },
    /// Associates a Crankshaft task with its name and TES id.
    Associate {
        /// The Crankshaft task id.
        id: u64,
        /// The name of the task.
        name: String,
        /// The TES id of the task.
        tes_id: String,
    },
    /// Removes a task from the monitor.
    Remove {
        /// The TES id of the task to remove.
        tes_id: String,
    },
}

/// A backend driven by the Task Execution Service (TES) schema.
#[derive(Debug)]
pub struct Backend {
    /// The unique name generator for tasks without names.
    names: Arc<Mutex<GeneratorIterator<UniqueAlphanumeric>>>,
    /// The state shared between tasks.
    state: Arc<State>,
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
    /// let backend = Backend::initialize(config, names, None);
    /// # });
    ///
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn initialize(
        config: Config,
        names: Arc<Mutex<GeneratorIterator<UniqueAlphanumeric>>>,
        events: Option<broadcast::Sender<Event>>,
    ) -> Self {
        let (url, http, interval) = config.into_parts();
        let mut builder = Client::builder().url(url);

        if let Some(auth) = &http.auth {
            builder = builder.insert_header("Authorization", auth.header_value());
        }

        let (tx, rx) = mpsc::channel(MONITOR_CAPACITY);

        let state = Arc::new(State {
            client: builder.try_build().expect("client to build"),
            interval: interval
                .map(Duration::from_secs)
                .unwrap_or(DEFAULT_INTERVAL),
            retries: http.retries.unwrap_or_default() as usize,
            policy: ExponentialFactorBackoff::from_millis(1000, 2.0).max_delay(MAX_RETRY_DELAY),
            monitor: tx,
            permits: Semaphore::new(
                http.max_concurrency
                    .unwrap_or(DEFAULT_MAX_CONCURRENT_REQUESTS),
            ),
            events,
        });

        // Spawn the monitor for monitoring task state
        // SAFETY: the name generator should _never_ run out of entries.
        let monitor_name = names.lock().unwrap().next().unwrap();
        tokio::spawn(Self::monitor(state.clone(), monitor_name, rx));

        Self {
            // SAFETY: the only required field of `builder` is the `url`, which
            // we provided earlier.
            names,
            state,
        }
    }

    /// Waits for a task to complete.
    async fn wait_task(
        state: &State,
        task_id: u64,
        task_name: &str,
        tes_id: &str,
        completed: oneshot::Receiver<Result<()>>,
    ) -> Result<NonEmpty<ExitStatus>, TaskRunError> {
        info!(
            "TES task `{tes_id}` (task `{task_name}`) has been created; waiting for task to start"
        );

        // Associate the TES task id with the Crankshaft id in the monitor
        state
            .monitor
            .send(MonitorRequest::Associate {
                id: task_id,
                name: task_name.to_string(),
                tes_id: tes_id.to_string(),
            })
            .await
            .context("failed to associate TES task id")?;

        // Wait for notification from the monitor that the task has completed
        completed
            .await
            .context("failed to wait for task completion")??;

        // Query for the state of the task
        let task = {
            let _permit = state
                .permits
                .acquire()
                .await
                .context("failed to acquire network request permit")?;

            state
                .client
                .get_task(
                    tes_id,
                    Some(&GetTaskParams { view: View::Full }),
                    state.policy(),
                )
                .await
                .context("failed to get task information from TES server")?
                .into_task()
                .context("returned task is not a full view")?
        };

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

                // There may be multiple task logs due to internal retries by the TES server
                // Therefore, we're only interested in the last log
                let logs = task.logs.unwrap_or_default();
                let task = logs.last().context(
                    "invalid response from TES server: completed task is missing task logs",
                )?;

                // Iterate the exit code from each executor log
                Ok(NonEmpty::collect(task.logs.iter().map(|executor| {
                    // See WEXITSTATUS from wait(2) to explain the shift
                    #[cfg(unix)]
                    let status = ExitStatus::from_raw(executor.exit_code << 8);

                    #[cfg(windows)]
                    let status = ExitStatus::from_raw(executor.exit_code as u32);

                    status
                }))
                .context(
                    "invalid response from TES server: completed task is missing executor logs",
                )?)
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

    /// Implements the task state monitor.
    ///
    /// The monitor first receives a request to add a new Crankshaft task for
    /// monitoring which contains a sender for notifying the task that it has
    /// completed.
    ///
    /// Once the TES task is created, another request is sent to associate the
    /// TES task id with the Crankshaft task id.
    ///
    /// The monitor periodically requests a list of tasks from the TES server.
    /// Tasks are grouped together by a tag that is
    async fn monitor(
        state: Arc<State>,
        monitor_name: String,
        mut rx: mpsc::Receiver<MonitorRequest>,
    ) {
        info!(
            "task monitor starting with polling interval of {interval} seconds",
            interval = state.interval.as_secs()
        );

        // The map of TES id to Crankshaft id
        let mut ids = HashMap::new();
        // The map of Crankshaft id to completion sender
        let mut senders = HashMap::new();
        // Set of known running tasks
        let mut running = HashSet::new();

        // The tag for the current group
        let mut group_tag = String::new();

        // The timer for the querying TES task state
        let mut timer = tokio::time::interval(state.interval);
        timer.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            select! {
                msg = rx.recv() => match msg {
                    Some(request) => match request {
                        MonitorRequest::Add { id, tag, completed } => {
                            // If the current set of senders is empty, create a new group tag
                            if senders.is_empty() {
                                ids.clear();
                                running.clear();
                                group_tag = format!(
                                    "{monitor_name}-{timestamp}-{id}",
                                    timestamp = SystemTime::now()
                                        .duration_since(SystemTime::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_secs()
                                );
                            }

                            // Notify the task of the tag that should be used for creating the TES task
                            let _ = tag.send(group_tag.clone());

                            // Insert the sender into the map
                            senders.insert(id, completed);
                        }
                        MonitorRequest::Associate { id, name, tes_id } => {
                            // Associate the TES id with the Crankshaft id
                            ids.insert(tes_id, (id, name));
                        }
                        MonitorRequest::Remove { tes_id } => {
                            if let Some((id, _)) = ids.remove(&tes_id) {
                                senders.remove(&id);
                            }
                        }
                    },
                    None => break,
                },
                _ = timer.tick() => {
                    // Don't do anything if there are no senders
                    if senders.is_empty() {
                        continue;
                    }

                    assert!(!group_tag.is_empty(), "should have a group id");
                    let mut page_token = None;
                    loop {
                        debug!("querying for the state of TES tasks with group tag `{group_tag}` and page token `{page_token:?}`");
                        let list = async {
                            let _permit = state
                                .permits
                                .acquire()
                                .await
                                .context("failed to acquire network request permit")?;

                            state
                                .client
                                .list_tasks(
                                    Some(&ListTasksParams {
                                        tag_keys: Some(vec![CRANKSHAFT_GROUP_TAG_NAME.to_string()]),
                                        tag_values: Some(vec![group_tag.clone()]),
                                        page_size: Some(MAX_PAGE_SIZE - 1),
                                        page_token,
                                        view: Some(View::Minimal),
                                        ..Default::default()
                                    }),
                                    state.policy(),
                                )
                                .await
                                .context("failed to get task information from TES server")
                        };

                        // Get the list of tasks
                        let result = list.await;

                        match result {
                            Ok(ListTasks {
                                tasks: tes_tasks,
                                next_page_token,
                            }) => {
                                // For any task that is completed and in the map, notify of completion
                                for task in tes_tasks
                                    .into_iter()
                                    .map(|t| t.into_minimal().expect("task should be minimal"))
                                {
                                    match task.state.unwrap_or_default() {
                                        TesState::Running | TesState::Paused => {
                                            // The task has completed, send the completion message
                                            if let Some((id, task_name)) = ids.get(&task.id) {
                                                if running.insert(*id) {
                                                    info!("TES task `{tes_id}` (task `{task_name}`) is now running", tes_id = task.id);
                                                    send_event!(state.events, Event::TaskStarted { id: *id });
                                                }
                                            }
                                        }
                                        TesState::Complete
                                        | TesState::ExecutorError
                                        | TesState::SystemError
                                        | TesState::Canceled
                                        | TesState::Preempted => {
                                            // The task has completed, send the completion message
                                            if let Some((id, _)) = ids.remove(&task.id) {
                                                running.remove(&id);
                                                if let Some(completed) = senders.remove(&id) {
                                                    let _ = completed.send(Ok(()));
                                                }
                                            }
                                        }
                                        _ => {}
                                    }
                                }

                                if next_page_token.is_none() {
                                    break;
                                }

                                page_token = next_page_token;
                            }
                            Err(e) => {
                                // Complete the current set of monitored tasks with an error
                                ids.clear();
                                running.clear();
                                for (_, completed) in senders.drain() {
                                    let _ = completed.send(Err(anyhow!("{e:#}")));
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }

        info!("task monitor has shut down");
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
        token: CancellationToken,
    ) -> Result<BoxFuture<'static, Result<NonEmpty<ExitStatus>, TaskRunError>>> {
        let task_id = next_task_id();
        let names = self.names.clone();
        let state = self.state.clone();
        Ok(async move {
            // Generate a name of the task if one wasn't provided
            let task_name = task.name.clone().unwrap_or_else(|| {
                // SAFETY: the name generator should _never_ run out of entries.
                names.lock().unwrap().next().unwrap()
            });

            let mut task = tes::v1::types::requests::Task::try_from(task)?;

            // Add the task to the monitor
            let (tag_tx, tag_rx) = oneshot::channel();
            let (completed_tx, completed_rx) = oneshot::channel();
            state
                .monitor
                .send(MonitorRequest::Add {
                    id: task_id,
                    tag: tag_tx,
                    completed: completed_tx,
                })
                .await
                .context("failed to add task to monitor")?;

            // Receive the tag to use from the monitor and insert it into the task
            let tag = tag_rx.await.context("failed to receive tag from monitor")?;

            task.tags
                .get_or_insert_default()
                .insert(CRANKSHAFT_GROUP_TAG_NAME.to_string(), tag);

            let tes_id = {
                let _permit = state
                    .permits
                    .acquire()
                    .await
                    .context("failed to acquire network request permit")?;

                select! {
                    // Always poll the cancellation token first
                    biased;
                    _ = token.cancelled() => {
                        return Err(TaskRunError::Canceled);
                    }
                    res = state.client.create_task(&task, state.policy()) => {
                        res.context("failed to create task with TES server")?.id
                    }
                }
            };

            let task_token = CancellationToken::new();

            send_event!(
                state.events,
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
                _ = task_token.cancelled() =>{
                    Err(TaskRunError::Canceled)
                }
                _ = token.cancelled() => {
                    Err(TaskRunError::Canceled)
                }
                res = Self::wait_task(&state, task_id, &task_name, &tes_id, completed_rx) => {
                    res
                }
            };

            if let Err(TaskRunError::Canceled) = &result {
                let _permit = state
                    .permits
                    .acquire()
                    .await
                    .context("failed to acquire permit")?;

                info!("canceling TES task `{tes_id}` (task `{task_name}`)");

                // Cancel the task
                state
                    .client
                    .cancel_task(&tes_id, state.policy())
                    .await
                    .context("failed to cancel task with TES server")?;
            }

            let _ = state.monitor.send(MonitorRequest::Remove { tes_id }).await;

            // Send an event for the result
            match &result {
                Ok(statuses) => send_event!(
                    state.events,
                    Event::TaskCompleted {
                        id: task_id,
                        exit_statuses: statuses.clone(),
                    }
                ),
                Err(TaskRunError::Canceled) => {
                    send_event!(state.events, Event::TaskCanceled { id: task_id })
                }
                Err(TaskRunError::Preempted) => {
                    send_event!(state.events, Event::TaskPreempted { id: task_id })
                }
                Err(TaskRunError::Other(e)) => send_event!(
                    state.events,
                    Event::TaskFailed {
                        id: task_id,
                        message: format!("{e:#}")
                    }
                ),
            }

            result
        }
        .boxed())
    }
}
