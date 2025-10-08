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
use crankshaft_events::Event;
use crankshaft_events::next_task_id;
use crankshaft_events::send_event;
use futures::FutureExt as _;
use futures::future::BoxFuture;
use nonempty::NonEmpty;
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
use tracing::info;

use super::TaskRunError;
use crate::Task;
use crate::service::name::GeneratorIterator;
use crate::service::name::UniqueAlphanumeric;
use crate::service::runner::backend::tes::monitor::TaskMonitor;

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
    /// The events sender for Crankshaft events.
    events: Option<broadcast::Sender<Event>>,
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
    /// let backend = Backend::initialize(config, names, None).await;
    /// # });
    ///
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub async fn initialize(
        config: Config,
        names: Arc<Mutex<GeneratorIterator<UniqueAlphanumeric>>>,
        events: Option<broadcast::Sender<Event>>,
    ) -> Self {
        let (url, http, interval) = config.into_parts();
        let mut builder = Client::builder().url(url);

        if let Some(auth) = &http.auth {
            builder = builder.insert_header("Authorization", auth.header_value());
        }

        let state = Arc::new(BackendState {
            // SAFETY: the only required field of `builder` is the `url`, which we provided earlier.
            client: builder.try_build().expect("client to build"),
            interval: interval
                .map(Duration::from_secs)
                .unwrap_or(DEFAULT_INTERVAL),
            retries: http.retries.unwrap_or_default() as usize,
            policy: ExponentialFactorBackoff::from_millis(1000, 2.0).max_delay(MAX_RETRY_DELAY),
            permits: Semaphore::new(
                http.max_concurrency
                    .unwrap_or(DEFAULT_MAX_CONCURRENT_REQUESTS),
            ),
            events,
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

    /// Waits for a task to complete.
    async fn wait_task(
        state: &BackendState,
        monitor: &TaskMonitor,
        task_id: u64,
        task_name: &str,
        tes_id: &str,
        completed: oneshot::Receiver<Result<()>>,
    ) -> Result<NonEmpty<ExitStatus>, TaskRunError> {
        info!(
            "TES task `{tes_id}` (task `{task_name}`) has been created; waiting for task to start"
        );

        // Associate the TES task id with the Crankshaft id in the monitor
        monitor.associate_task_id(task_id, tes_id.to_string()).await;

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
            let tag = monitor.add_task(task_id, task_name.clone(), completed_tx).await;

            task.tags
                .get_or_insert_default()
                .insert(monitor::CRANKSHAFT_GROUP_TAG_NAME.to_string(), tag);

            let permit = state
                .permits
                .acquire()
                .await
                .context("failed to acquire network request permit")?;

            let tes_id = select! {
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
                res = Self::wait_task(&state, &monitor, task_id, &task_name, &tes_id, completed_rx) => {
                    res
                }
            };

            if let Err(TaskRunError::Canceled) = &result {
                let permit = state
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

                // Drop the permit now that the request has completed
                drop(permit);
            }

            monitor.remove_task(tes_id).await;

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
