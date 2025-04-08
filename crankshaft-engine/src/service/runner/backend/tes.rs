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

use async_trait::async_trait;
use crankshaft_config::backend::tes::Config;
use eyre::Context;
use eyre::Result;
use eyre::bail;
use futures::FutureExt as _;
use futures::future::BoxFuture;
use nonempty::NonEmpty;
use tes::v1::Client;
use tes::v1::client::tasks::View;
use tes::v1::types::task::State;
use tokio::select;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use tracing::info;
use tracing::trace;

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
        let mut builder = Client::builder().url(config.url);

        if let Some(token) = config.http.basic_auth_token {
            builder = builder.insert_header("Authorization", format!("Basic {}", token));
        }

        Self {
            // SAFETY: the only required field of `builder` is the `url`, which
            // we provided earlier.
            client: Arc::new(builder.try_build().expect("client to build")),
            interval: config
                .interval
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
    ) -> Result<NonEmpty<ExitStatus>> {
        info!("TES task `{task_id}` (task `{name}`) has been created; waiting for task to start");

        loop {
            let task = client
                .get_task(task_id, View::Minimal)
                .await
                .context("failed to get task information")?
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
                    State::Complete | State::ExecutorError | State::SystemError => {
                        // Repeat with a basic request
                        let task = client
                            .get_task(task_id, View::Basic)
                            .await
                            .context("failed to get task information")?
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

                        let mut statuses = task
                            .logs
                            .unwrap()
                            .into_iter()
                            .flat_map(|task| task.logs)
                            .map(|log| {
                                let status = log.exit_code.expect("exit code to be present");

                                // See WEXITSTATUS from wait(2) to explain the shift
                                #[cfg(unix)]
                                let status = ExitStatus::from_raw((status as i32) << 8);

                                #[cfg(windows)]
                                let status = ExitStatus::from_raw(status);

                                status
                            });

                        // SAFETY: at least one set of logs is always
                        // expected to be returned from the server.
                        let mut result = NonEmpty::new(statuses.next().unwrap());
                        result.extend(statuses);
                        return Ok(result);
                    }
                    State::Canceled => bail!("task has been cancelled"),
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
    ) -> Result<BoxFuture<'static, Result<NonEmpty<ExitStatus>>>> {
        let client = self.client.clone();
        let name = task.name.clone();
        let task: tes::v1::types::Task = tes::v1::types::Task::try_from(task)?;
        let interval = self.interval;

        Ok(async move {
            let task_id = client.create_task(task).await?.id;

            select! {
                // Always poll the cancellation token first
                biased;

                _ = token.cancelled() => {
                    // Cancel the task
                    client.cancel_task(&task_id).await?;
                    bail!("task has been cancelled")
                }
                res = Self::wait_task(&client, &task_id, name.as_deref().unwrap_or("<unnamed>"), interval, started) => {
                    res
                }
            }
        }
        .boxed())
    }
}
