//! A generic backend.
//!
//! Generic backends are intended to be relatively malleable and configurable by
//! the end user without requiring the need to write Rust code.

use std::borrow::Cow;
use std::process::ExitStatus;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use crankshaft_config::backend::Defaults;
use crankshaft_config::backend::generic::Config;
use crankshaft_config::backend::generic::SubValue;
use futures::FutureExt;
use futures::future::BoxFuture;
use nonempty::NonEmpty;
use regex::Regex;
use tokio::select;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::error;
use tracing::trace;
use tracing::warn;

use super::TaskRunError;
use crate::Task;
use crate::service::runner::backend::generic::driver::Driver;
use crate::task::Resources;

pub mod driver;

/// The default number of seconds to wait between monitor commands.
pub const DEFAULT_MONITOR_FREQUENCY: u64 = 5;

/// The generic backend.
#[derive(Debug)]
pub struct Backend {
    /// The driver.
    driver: Arc<Driver>,

    /// The inner configuration.
    config: Config,

    /// The execution defaults.
    defaults: Option<Defaults>,
}

impl Backend {
    /// Attempts to initialize a new generic [`Backend`] with the default
    /// connection settings and the provided configuration for the backend.
    pub async fn initialize(config: Config, defaults: Option<Defaults>) -> Result<Self> {
        // TODO(clay): this could be "taken" instead to avoid the clone.
        let driver = Driver::initialize(config.driver().clone())
            .await
            .map(Arc::new)?;

        Ok(Self {
            driver,
            config,
            defaults,
        })
    }

    /// Gets the inner configuration.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Gets the inner driver.
    pub fn driver(&self) -> &Driver {
        &self.driver
    }

    /// Resolves the resources for a particular task.
    // NOTE: first, the default resources from the code are assumed. Then, the
    // default resources from the configuration are applied (if they are
    // provided). Last, the resources from the execution itself are applied.
    // This is the relative level of priority resource resolution should have,
    // and the order is important to preserve.
    fn resolve_resources(&self, task: Option<&Resources>) -> Option<Resources> {
        let mut resources: Option<Resources> = None;

        if let Some(defaults) = &self.defaults {
            let defaults = Resources::from(defaults);
            resources = Some(resources.unwrap_or_default().apply(&defaults));
        }

        if let Some(task) = task {
            resources = Some(resources.unwrap_or_default().apply(task));
        }

        resources
    }
}

impl crate::Backend for Backend {
    /// Gets the default name for the backend.
    fn default_name(&self) -> &'static str {
        "generic"
    }

    /// Runs a task in a backend.
    fn run(
        &self,
        task: Task,
        mut started: Option<oneshot::Sender<()>>,
        token: CancellationToken,
    ) -> Result<BoxFuture<'static, Result<NonEmpty<ExitStatus>, TaskRunError>>> {
        let driver = self.driver.clone();
        let config = self.config.clone();

        let mut default_substitutions = self
            .resolve_resources(task.resources.as_ref())
            .map(|resources| resources.to_hashmap())
            .unwrap_or_default();
        for input in task.inputs() {
            let inputs = default_substitutions
                .entry(Cow::Borrowed("inputs"))
                .or_insert_with(|| SubValue::Array(vec![]))
                .as_array_mut()
                .expect("inputs array was not an array");
            let host_path = match input.contents() {
                crate::task::input::Contents::Url(_) => unimplemented!(),
                crate::task::input::Contents::Literal(_) => unimplemented!(),
                crate::task::input::Contents::Path(path) => path.display().to_string(),
            };
            let pair = serde_json::json!({
                "host_path": host_path,
                "guest_path": input.path()
            });
            inputs.push(pair);
        }
        for output in task.outputs() {
            let outputs = default_substitutions
                .entry(Cow::Borrowed("outputs"))
                .or_insert_with(|| SubValue::Array(vec![]))
                .as_array_mut()
                .expect("outputs array was not an array");
            if output.url().scheme() != "file" {
                unimplemented!("non-file outputs unsupported");
            };
            let host_path = output
                .url()
                .to_file_path()
                .expect("failed to make output url into a path");
            let pair = serde_json::json!({
                "host_path": host_path,
                "guest_path": output.path()
            });
            outputs.push(pair);
        }

        Ok(async move {
            let mut statuses = Vec::new();
            let job_id_regex = config
                .job_id_regex()
                .as_ref()
                .map(|pattern| {
                    Regex::new(pattern)
                        .with_context(|| format!("job regex `{pattern}` is not valid"))
                })
                .transpose()?;

            for execution in task.executions {
                if token.is_cancelled() {
                    return Err(TaskRunError::Canceled);
                }

                // TODO(clay): this will warn every time for now. We need to
                // change the model of how tasks are done internally to remove
                // this need.
                warn!(
                    "generic backends do not support images; as such, the directive to use a `{}` \
                     image will be ignored",
                    execution.image
                );

                let mut substitutions = default_substitutions.clone();

                if substitutions
                    .insert(
                        "command".into(),
                        shlex::try_join(
                            std::iter::once(execution.program.as_str())
                                .chain(execution.args.iter().map(String::as_str)),
                        )
                        .map_err(|e| TaskRunError::Other(e.into()))?
                        .into(),
                    )
                    .is_some()
                {
                    unreachable!("the `command` key should not be present here");
                };

                if let Some(cwd) = &execution.work_dir {
                    if substitutions.insert("cwd".into(), cwd.as_str().into()).is_some() {
                        unreachable!("the `cwd` key should not be present here");
                    };
                }
                if let Some(stdout) = &execution.stdout() {
                    substitutions.insert("stdout".into(), (*stdout).into());
                }
                if let Some(stderr) = &execution.stderr() {
                    substitutions.insert("stderr".into(), (*stderr).into());
                }

                // Submitting the initial job.
                let submit = config
                    .resolve_submit(&substitutions)
                    .context("failed to resolve submit command")?;
                let output = driver
                    .run(submit)
                    .await
                    .context("failed to run submit command")?;
                if !output.status.success() {
                    error!(status = ?output.status, "submit command failed");
                    debug!(stdout = %String::from_utf8_lossy(&output.stdout), "submit command failed");
                    debug!(stderr = %String::from_utf8_lossy(&output.stderr), "submit command failed");
                    return Err(anyhow!("submit command failed: {}", output.status).into());
                }

                // Notify that execution has started
                if let Some(started) = started.take() {
                    started.send(()).ok();
                }

                // Monitoring the output.
                match job_id_regex {
                    Some(ref regex) => {
                        trace!(regex = regex.as_str(), "looking for job_id with regex");
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let captures = regex.captures_iter(&stdout).next().unwrap_or_else(|| {
                            panic!(
                                "could not match the job id regex within stdout: `{}`",
                                stdout
                            )
                        });

                        // SAFETY: this will always unwrap, as the group is
                        // _required_ for the pattern to match.
                        let id = captures.get(1).map(|c| c.as_str()).unwrap();
                        trace!(id, "job_id found");
                        substitutions.insert("job_id".into(), id.into());

                        loop {
                            let monitor = config
                                .resolve_monitor(&substitutions)
                                .context("failed to resolve monitor command")?;

                            let result = select! {
                                // Always poll the cancellation token first
                                biased;

                                _ = token.cancelled() => {
                                    Err(TaskRunError::Canceled)
                                }
                                res = driver.run(monitor) => {
                                    res.map_err(TaskRunError::Other)
                                }
                            };

                            // Run the kill command when canceled
                            if token.is_cancelled() {
                                let kill = config
                                    .resolve_kill(&substitutions)
                                    .context("failed to resolve kill command")?;
                                driver
                                    .run(kill)
                                    .await
                                    .context("failed to run kill command")?;
                            }

                            let output = result?;
                            if output.status.success() {
                                let get_exit_code = config
                                    .resolve_get_exit_code(&substitutions)
                                    .context("failed to resolve get_exit_code command")?;
                                let get_exit_code_out = driver
                                    .run(get_exit_code)
                                    .await
                                    .context("failed to run get_exit_code command")?;
                                let get_exit_code_stdout =
                                    String::from_utf8(get_exit_code_out.stdout)
                                        .context("exit code output was not valid UTF-8")?
                                        .trim()
                                        .to_owned();
                                trace!(get_exit_code_stdout);
                                cfg_if::cfg_if! {
                                    if #[cfg(unix)] {
                                        use std::os::unix::process::ExitStatusExt;
                                        let job_status = ExitStatus::from_raw(
                                            get_exit_code_stdout.parse::<i32>()
                                                .context("exit code output was not a valid i32")? << 8
                                        );
                                    } else {
                                        compile_error!("unsupported platform")
                                    }
                                }
                                statuses.push(job_status);
                                break;
                            }

                            tokio::time::sleep(Duration::from_secs(
                                config
                                    .monitor_frequency()
                                    .unwrap_or(DEFAULT_MONITOR_FREQUENCY),
                            ))
                            .await;
                        }
                    }
                    _ => {
                        statuses.push(output.status);
                    }
                }
            }

            // SAFETY: each task _must_ have at least one execution, so at least one
            // execution result _must_ exist at this stage. Thus, this will always unwrap.
            Ok(NonEmpty::from_vec(statuses).unwrap())
        }
        .boxed())
    }
}
