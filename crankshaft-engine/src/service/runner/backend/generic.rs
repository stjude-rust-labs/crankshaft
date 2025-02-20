//! A generic backend.
//!
//! Generic backends are intended to be relatively malleable and configurable by
//! the end user without requiring the need to write Rust code.

use std::process::Output;
use std::sync::Arc;
use std::time::Duration;

use crankshaft_config::backend::Defaults;
use crankshaft_config::backend::generic::Config;
use eyre::Context as _;
use eyre::bail;
use eyre::eyre;
use futures::FutureExt;
use futures::future::BoxFuture;
use nonempty::NonEmpty;
use regex::Regex;
use tokio::select;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use tracing::warn;

use crate::Result;
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
    ) -> Result<BoxFuture<'static, Result<NonEmpty<Output>>>> {
        let driver = self.driver.clone();
        let config = self.config.clone();

        let default_substitutions = self
            .resolve_resources(task.resources())
            .map(|resources| resources.to_hashmap())
            .unwrap_or_default();

        Ok(async move {
            let mut outputs = Vec::new();
            let job_id_regex = config
                .job_id_regex()
                .map(|pattern| {
                    Regex::new(pattern)
                        .with_context(|| format!("job regex `{pattern}` is not valid"))
                })
                .transpose()?;

            for execution in task.executions() {
                if token.is_cancelled() {
                    bail!("task has been cancelled");
                }

                // TODO(clay): this will warn every time for now. We need to
                // change the model of how tasks are done internally to remove
                // this need.
                warn!(
                    "generic backends do not support images; as such, the directive to use a `{}` \
                     image will be ignored",
                    execution.image()
                );

                let mut substitutions = default_substitutions.clone();

                if substitutions
                    .insert(
                        "command".into(),
                        shlex::try_join(
                            std::iter::once(execution.program())
                                .chain(execution.args().iter().map(String::as_str)),
                        )?
                        .into(),
                    )
                    .is_some()
                {
                    unreachable!("the `command` key should not be present here");
                };

                if let Some(cwd) = execution.work_dir() {
                    if substitutions.insert("cwd".into(), cwd.into()).is_some() {
                        unreachable!("the `cwd` key should not be present here");
                    };
                }

                // Submitting the initial job.
                let submit = config
                    .resolve_submit(&substitutions)
                    .context("failed to resolve submit command")?;
                let output = driver
                    .run(submit)
                    .await
                    .context("failed to run submit command")?;

                // Notify that execution has started
                if let Some(started) = started.take() {
                    started.send(()).ok();
                }

                // Monitoring the output.
                match job_id_regex {
                    Some(ref regex) => {
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
                        substitutions.insert("job_id".into(), id.into());

                        loop {
                            let monitor = config
                                .resolve_monitor(&substitutions)
                                .context("failed to resolve monitor command")?;

                            let result = select! {
                                // Always poll the cancellation token first
                                biased;

                                _ = token.cancelled() => {
                                    Err(eyre!("task has been cancelled"))
                                }
                                res = driver.run(monitor) => {
                                    res
                                }
                            };

                            // Run the kill command when cancelled
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

                            if !output.status.success() {
                                outputs.push(output);
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
                        outputs.push(output);
                    }
                }
            }

            // SAFETY: each task _must_ have at least one execution, so at least one
            // execution result _must_ exist at this stage. Thus, this will always unwrap.
            Ok(NonEmpty::from_vec(outputs).unwrap())
        }
        .boxed())
    }
}
