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
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use anyhow::anyhow;
use async_trait::async_trait;
use crankshaft_config::backend::tes::Config;
use futures::FutureExt as _;
use futures::future::BoxFuture;
use nonempty::NonEmpty;
use tes::v1::Client;
use tes::v1::types::requests::GetTaskParams;
use tes::v1::types::requests::View;
use tes::v1::types::task::State;
use tokio::select;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use tracing::info;
use tracing::trace;

use super::TaskRunError;
use crate::Task;

/// The default poll interval for querying task status.
const DEFAULT_INTERVAL: Duration = Duration::from_secs(1);

/// A backend driven by the Task Execution Service (TES) schema.
#[derive(Debug)]
pub struct Backend {
    /// A handle to the inner TES client.
    client: Arc<Client>,
    /// The poll interval for checking on task status.
    interval: Duration,
}

impl Backend {
    /// AttemptsCreates a new [`Backend`].
    ///
    /// # Examples
    ///
    /// ```
    /// use crankshaft_config::backend::tes::Config;
    /// use crankshaft_engine::service::runner::backend::tes::Backend;
    /// use url::Url;
    ///
    /// let url = "http://localhost:8000".parse::<Url>()?;
    /// let config = Config::builder().url(url).build();
    ///
    /// let backend = Backend::initialize(config);
    ///
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn initialize(config: Config) -> Self {
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
        }
    }

    /// Waits for a task to complete.
    async fn wait_task(
        client: &Client,
        task_id: &str,
        name: &str,
        interval: Duration,
        mut started: Option<oneshot::Sender<()>>,
    ) -> Result<NonEmpty<ExitStatus>, TaskRunError> {
        info!("TES task `{task_id}` (task `{name}`) has been created; waiting for task to start");

        loop {
            let task = client
                .get_task(
                    task_id,
                    Some(&GetTaskParams {
                        view: View::Minimal,
                    }),
                )
                .await
                .context("failed to get task information from TES server")?
                .into_minimal()
                .unwrap();

            trace!("response for `{task_id}`: {task:?}");

            if let Some(ref state) = task.state {
                match state {
                    State::Unknown | State::Queued | State::Initializing => {
                        // Task hasn't started yet
                        trace!("task `{task_id}` is not yet running; waiting before polling again");
                    }
                    State::Running | State::Paused => {
                        trace!("task `{task_id}` is running; waiting before polling again");

                        // Task is running (or was previously running but now paused), so notify
                        if let Some(started) = started.take() {
                            info!("TES task `{task_id}` (task `{name}`) has started");
                            started.send(()).ok();
                        }
                    }
                    State::Canceling => {
                        // Task is canceling, wait for it to cancel
                        trace!("task `{task_id}` is canceling; waiting before polling again");
                    }
                    State::SystemError => {
                        // Repeat with a full request to get the system logs.
                        let task = client
                            .get_task(task_id, Some(&GetTaskParams { view: View::Full }))
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
                            "task failed due to system error:\n\n{messages}",
                        )));
                    }
                    State::Complete | State::ExecutorError => {
                        // Repeat with a basic request to get executor logs
                        let task = client
                            .get_task(task_id, Some(&GetTaskParams { view: View::Basic }))
                            .await
                            .context("failed to get task information from TES server")?
                            .into_task()
                            .unwrap();

                        // Task completed or had an error
                        if *state == State::Complete {
                            info!("TES task `{task_id}` (task `{name}`) has completed");
                        } else {
                            info!("TES task `{task_id}` (task `{name}`) has failed");
                        }

                        // Task has completed, so notify that it started if we haven't already
                        if let Some(started) = started.take() {
                            started.send(()).ok();
                        }

                        // There may be multiple task logs due to internal retries by the TES server
                        // Therefore, we're only interested in the last log
                        let logs = task.logs.unwrap_or_default();
                        let task = logs.last().context(
                            "invalid response from TES server: completed task is missing task logs",
                        )?;

                        // Iterate the exit code from each executor log
                        let mut statuses = task.logs.iter().map(|executor| {
                            // See WEXITSTATUS from wait(2) to explain the shift
                            #[cfg(unix)]
                            let status = ExitStatus::from_raw(executor.exit_code << 8);

                            #[cfg(windows)]
                            let status = ExitStatus::from_raw(executor.exit_code as u32);

                            status
                        });

                        let mut result = NonEmpty::new(statuses.next().context(
                            "invalid response from TES server: completed task is missing executor \
                             logs",
                        )?);
                        result.extend(statuses);
                        return Ok(result);
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
        started: Option<oneshot::Sender<()>>,
        token: CancellationToken,
    ) -> Result<BoxFuture<'static, Result<NonEmpty<ExitStatus>, TaskRunError>>> {
        let client = self.client.clone();
        let name = task.name.clone();
        let task = tes::v1::types::requests::Task::try_from(task)?;
        let interval = self.interval;

        Ok(async move {
            let task_id = client
                .create_task(&task)
                .await
                .context("failed to create task with TES server")?
                .id;

            select! {
                // Always poll the cancellation token first
                biased;

                _ = token.cancelled() => {
                    // Cancel the task
                    client
                        .cancel_task(&task_id)
                        .await
                        .context("failed to cancel task with TES server")?;
                    Err(TaskRunError::Canceled)
                }
                res = Self::wait_task(&client, &task_id, name.as_deref().unwrap_or("<unnamed>"), interval, started) => {
                    res
                }
            }
        }
        .boxed())
    }
}
