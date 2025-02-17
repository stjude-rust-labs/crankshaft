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
use std::process::Output;
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
use tracing::debug;
use tracing::trace;

use crate::Task;

/// A backend driven by the Task Execution Service (TES) schema.
#[derive(Debug)]
pub struct Backend {
    /// A handle to the inner TES client.
    client: Arc<Client>,
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
        let (url, config) = config.into_parts();

        let mut builder = Client::builder().url(url);

        if let Some(token) = config.basic_auth_token() {
            builder = builder.insert_header("Authorization", format!("Basic {}", token));
        }

        Self {
            // SAFETY: the only required field of `builder` is the `url`, which
            // we provided earlier.
            client: Arc::new(builder.try_build().expect("client to build")),
        }
    }

    /// Waits for a task to complete.
    async fn wait_task(
        client: &Client,
        task_id: &str,
        started: oneshot::Sender<()>,
    ) -> Result<NonEmpty<Output>> {
        let mut started = Some(started);

        loop {
            let task = client
                .get_task(task_id, View::Full)
                .await
                .context("failed to get task information")?;

            trace!("response for {task_id}: {task:?}");

            // SAFETY: `get_task` called with `View::Full` will always
            // return a full [`Task`], so this will always unwrap.
            let task = task.into_task().unwrap();

            if let Some(ref state) = task.state {
                debug!("state for task {task_id}: {state:?}");
                match state {
                    State::Unknown | State::Queued | State::Initializing => {
                        // Task hasn't started yet
                        debug!("task is not yet running; waiting before polling again");
                    }
                    State::Running | State::Paused => {
                        debug!("task is running; waiting before polling again");

                        // Task is running (or was previously running but now paused), so notify
                        if let Some(started) = started.take() {
                            started.send(()).ok();
                        }
                    }
                    State::Complete | State::ExecutorError | State::SystemError => {
                        // Task completed or had an error
                        if *state == State::Complete {
                            debug!("task has completed");
                        } else {
                            debug!("task has failed");
                        }

                        // Task has completed, so notify that it started if we haven't already
                        if let Some(started) = started.take() {
                            started.send(()).ok();
                        }

                        let mut results = task
                            .logs
                            .unwrap()
                            .into_iter()
                            .flat_map(|task| task.logs)
                            .map(|log| {
                                let status = log.exit_code.expect("exit code to be present");

                                #[cfg(unix)]
                                let output = Output {
                                    status: ExitStatus::from_raw(status as i32),
                                    stdout: log.stdout.unwrap_or_default().as_bytes().to_vec(),
                                    stderr: log.stderr.unwrap_or_default().as_bytes().to_vec(),
                                };

                                #[cfg(windows)]
                                let output = Output {
                                    status: ExitStatus::from_raw(status),
                                    stdout: log.stdout.unwrap_or_default().as_bytes().to_vec(),
                                    stderr: log.stderr.unwrap_or_default().as_bytes().to_vec(),
                                };

                                output
                            });

                        // SAFETY: at least one set of logs is always
                        // expected to be returned from the server.
                        // TODO(clay): we should probably change this to a
                        // recoverable error.
                        let mut outputs = NonEmpty::new(results.next().unwrap());
                        outputs.extend(results);

                        return Ok(outputs);
                    }
                    State::Canceled => bail!("task has been cancelled"),
                }
            }

            // TODO(clay): make this configurable.
            tokio::time::sleep(Duration::from_millis(200)).await;
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
        started: oneshot::Sender<()>,
        token: CancellationToken,
    ) -> Result<BoxFuture<'static, Result<NonEmpty<Output>>>> {
        let client = self.client.clone();
        let task = tes::v1::types::Task::try_from(task)?;

        Ok(async move {
            let task_id = client.create_task(task).await?.id;

            select! {
                _ = token.cancelled() => {
                    // Cancel the task
                    client.cancel_task(&task_id).await?;
                    bail!("task has been cancelled")
                }
                res = Self::wait_task(&client, &task_id, started) => {
                    res
                }
            }
        }
        .boxed())
    }
}
