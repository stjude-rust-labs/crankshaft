//! A generic backend.
//!
//! Generic backends are intended to be relatively maleable and configurable by
//! the end user without requiring the need to write Rust code.

use std::sync::Arc;
use std::time::Duration;

use crankshaft_config::backend::Defaults;
use crankshaft_config::backend::generic::Config;
use eyre::Context as _;
use futures::FutureExt;
use futures::future::BoxFuture;
use nonempty::NonEmpty;
use regex::Regex;
use tracing::warn;

use crate::Result;
use crate::Task;
use crate::service::runner::backend::TaskResult;
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
    fn run(&self, task: Task) -> BoxFuture<'static, TaskResult> {
        let driver = self.driver.clone();
        let config = self.config.clone();

        let default_substitutions = self
            .resolve_resources(task.resources())
            .and_then(|resources| resources.to_hashmap())
            .unwrap_or_default();

        async move {
            let mut outputs = Vec::new();
            let job_id_regex = config.job_id_regex().map(|pattern| {
                Regex::new(pattern)
                    .context("compiling job id regex")
                    .unwrap()
            });

            for execution in task.executions() {
                // TODO(clay): this will warn every time for now. We need to
                // change the model of how tasks are done internally to remove
                // this need.
                warn!(
                    "generic backends do not support images; as such, the directive to use a `{}` \
                     image will be ignored",
                    execution.image()
                );

                // TODO(clay): surely we can do better than a reallocation here.
                let shell = execution
                    .args()
                    .into_iter()
                    .map(String::from)
                    .collect::<Vec<String>>()
                    .join(" ");

                let mut subtitutions = default_substitutions.clone();

                if subtitutions.insert(String::from("shell"), shell).is_some() {
                    unreachable!("the `shell` key should not be present here");
                };

                if let Some(cwd) = execution.workdir() {
                    if subtitutions
                        .insert(String::from("cwd"), cwd.into())
                        .is_some()
                    {
                        unreachable!("the `cwd` key should not be present here");
                    };
                }

                // (1) Submitting the initial job.
                // TODO(clay): we should probably handle this more gracefully.
                let submit = config.resolve_submit(&subtitutions).unwrap();

                // TODO(clay): we should probably handle this more gracefully.
                let output = driver.run(submit).await.unwrap();

                // (2) Monitoring the output.
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
                        let id = captures.get(1).map(|c| String::from(c.as_str())).unwrap();
                        subtitutions.insert(String::from("job_id"), id);

                        loop {
                            let monitor = config.resolve_monitor(&subtitutions).unwrap();
                            let output = driver.run(monitor).await.unwrap();

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

            let mut outputs = outputs.into_iter();

            // SAFETY: each task _must_ have at least one execution, so at least one
            // execution result _must_ exist at this stage. Thus, this will always unwrap.
            let mut executions = NonEmpty::new(outputs.next().unwrap());
            executions.extend(outputs);

            TaskResult { executions }
        }
        .boxed()
    }
}
