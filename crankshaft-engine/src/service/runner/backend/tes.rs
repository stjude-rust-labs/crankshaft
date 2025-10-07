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
use tes::v1::types::task::State;
use tokio::select;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use tracing::info;
use tracing::trace;

use super::TaskRunError;
use crate::Task;
use crate::service::name::GeneratorIterator;
use crate::service::name::UniqueAlphanumeric;

/// The default poll interval for querying task status.
const DEFAULT_INTERVAL: Duration = Duration::from_secs(1);

/// A backend driven by the Task Execution Service (TES) schema.
#[derive(Debug)]
pub struct Backend {
    /// A handle to the inner TES client.
    client: Arc<Client>,
    /// The poll interval for checking on task status.
    interval: Duration,
    /// The events sender for the backend.
    events: Option<broadcast::Sender<Event>>,
    /// The unique name generator for tasks without names.
    names: Arc<Mutex<GeneratorIterator<UniqueAlphanumeric>>>,
    /// The number of retries to attempt.
    retries: usize,
    /// The retry policy to use for client operations.
    policy: ExponentialFactorBackoff,
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
    /// let backend = Backend::initialize(config, names, None);
    ///
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn initialize(
        config: Config,
        names: Arc<Mutex<GeneratorIterator<UniqueAlphanumeric>>>,
        events: Option<broadcast::Sender<Event>>,
    ) -> Self {
        let (url, config, interval) = config.into_parts();
        let mut builder = Client::builder().url(url);

        if let Some(auth) = &config.auth {
            builder = builder.insert_header("Authorization", auth.header_value());
        }

        Self {
            // SAFETY: the only required field of `builder` is the `url`, which
            // we provided earlier.
            client: Arc::new(builder.try_build().expect("client to build")),
            interval: interval
                .map(Duration::from_secs)
                .unwrap_or(DEFAULT_INTERVAL),
            events,
            names,
            retries: config.retries.unwrap_or_default() as usize,
            policy: ExponentialFactorBackoff::from_millis(1000, 2.0)
                .max_delay(Duration::from_secs(10)),
        }
    }

    /// Waits for a task to complete.
    async fn wait_task(
        client: &Client,
        task_id: u64,
        task_name: &str,
        tes_id: &str,
        interval: Duration,
        events: Option<broadcast::Sender<Event>>,
        retries: impl Iterator<Item = Duration> + Clone,
    ) -> Result<NonEmpty<ExitStatus>, TaskRunError> {
        info!(
            "TES task `{tes_id}` (task `{task_name}`) has been created; waiting for task to start"
        );

        loop {
            let task = client
                .get_task(
                    tes_id,
                    Some(&GetTaskParams {
                        view: View::Minimal,
                    }),
                    retries.clone(),
                )
                .await
                .context("failed to get task information from TES server")?
                .into_minimal()
                .unwrap();

            trace!("response for TES task `{tes_id}`: {task:?}");

            if let Some(ref state) = task.state {
                match state {
                    State::Unknown | State::Queued | State::Initializing => {
                        // Task hasn't started yet
                        trace!(
                            "TES task `{tes_id}` is not yet running; waiting before polling again"
                        );
                    }
                    State::Running | State::Paused => {
                        trace!("TES task `{tes_id}` is running; waiting before polling again");
                        send_event!(events, Event::TaskStarted { id: task_id });
                    }
                    State::Canceling => {
                        // Task is canceling, wait for it to cancel
                        trace!("TES task `{tes_id}` is canceling; waiting before polling again");
                    }
                    State::SystemError => {
                        // Repeat with a full request to get the system logs.
                        let task = client
                            .get_task(
                                tes_id,
                                Some(&GetTaskParams { view: View::Full }),
                                retries.clone(),
                            )
                            .await
                            .context("failed to get task information from TES server")?
                            .into_task()
                            .unwrap();

                        let messages = task
                            .logs
                            .unwrap_or_default()
                            .last()
                            .and_then(|l| l.system_logs.as_ref().map(|l| l.join("\n")))
                            .unwrap_or_default();

                        return Err(TaskRunError::Other(anyhow!(
                            "task failed due to system error:\n\n{messages}"
                        )));
                    }
                    State::Complete => {
                        // Repeat with a basic request to get executor logs
                        let task = client
                            .get_task(
                                tes_id,
                                Some(&GetTaskParams { view: View::Basic }),
                                retries.clone(),
                            )
                            .await
                            .context("failed to get task information from TES server")?
                            .into_task()
                            .unwrap();

                        info!("TES task `{tes_id}` (task `{task_name}`) has completed");

                        // There may be multiple task logs due to internal retries by the TES server
                        // Therefore, we're only interested in the last log
                        let logs = task.logs.unwrap_or_default();
                        let task = logs.last().context(
                            "invalid response from TES server: completed task is missing task logs",
                        )?;

                        // Iterate the exit code from each executor log
                        return Ok(NonEmpty::collect(task.logs.iter().map(|executor| {
                            // See WEXITSTATUS from wait(2) to explain the shift
                            #[cfg(unix)]
                            let status = ExitStatus::from_raw(executor.exit_code << 8);

                            #[cfg(windows)]
                            let status = ExitStatus::from_raw(executor.exit_code as u32);

                            status
                        }))
                        .context(
                            "invalid response from TES server: completed task is missing executor \
                             logs",
                        )?);
                    }
                    State::ExecutorError => {
                        info!("TES task `{tes_id}` (task `{task_name}`) has failed");
                        return Err(TaskRunError::Other(anyhow!(
                            "task failed due to executor error"
                        )));
                    }
                    State::Canceled => return Err(TaskRunError::Canceled),
                    State::Preempted => return Err(TaskRunError::Preempted),
                }
            }

            tokio::time::sleep(interval).await;
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
        let client = self.client.clone();
        let interval = self.interval;
        let events = self.events.clone();
        let names = self.names.clone();
        let retries = self.policy.clone().take(self.retries);

        let task_token = CancellationToken::new();

        Ok(async move {
            // Generate a name of the task if one wasn't provided
            let task_name = task.name.clone().unwrap_or_else(|| {
                let mut generator = names.lock().unwrap();
                // SAFETY: the name generator should _never_ run out of entries.
                generator.next().unwrap()
            });

            let task = tes::v1::types::requests::Task::try_from(task)?;

            let tes_id = client
                .create_task(&task, retries.clone())
                .await
                .context("failed to create task with TES server")?
                .id;

            send_event!(events, Event::TaskCreated { id: task_id, name: task_name.clone(), tes_id: Some(tes_id.clone()), token: task_token.clone()});

            let result = select! {
                // Always poll the cancellation token first
                biased;
                _ = task_token.cancelled() =>{
                    Err(TaskRunError::Canceled)
                }
                _ = token.cancelled() => {
                    // Cancel the task
                    client
                        .cancel_task(&tes_id, retries.clone())
                        .await
                        .context("failed to cancel task with TES server")?;
                    Err(TaskRunError::Canceled)
                }
                res = Self::wait_task(&client, task_id, &task_name, &tes_id, interval, events.clone(), retries.clone()) => {
                    res
                }
            };

            // Send an event for the result
            match &result {
                Ok(statuses) => send_event!(
                    events,
                    Event::TaskCompleted {
                        id: task_id,
                        exit_statuses: statuses.clone(),
                    }
                ),
                Err(TaskRunError::Canceled) => send_event!(
                    events,
                    Event::TaskCanceled {
                        id: task_id
                    }
                ),
                Err(TaskRunError::Preempted) => send_event!(
                    events,
                    Event::TaskPreempted {
                        id: task_id
                    }
                ),
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
        .boxed())
    }
}
