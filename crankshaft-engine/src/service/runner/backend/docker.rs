//! A Docker backend.

use std::process::Output;
use std::sync::Arc;

use async_trait::async_trait;
use bollard::secret::HostConfig;
use bollard::secret::Mount;
use bollard::secret::MountTypeEnum;
use crankshaft_config::backend::docker::Config;
use crankshaft_docker::Docker;
use eyre::Context;
use eyre::ContextCompat;
use futures::FutureExt;
use futures::future::BoxFuture;
use nonempty::NonEmpty;
use tempfile::TempDir;

use crate::Result;
use crate::Task;

/// A local execution backend.
#[derive(Debug)]
pub struct Backend {
    /// A handle to the inner docker client.
    client: Docker,
    /// Configuration for the backend.
    config: Config,
}

impl Backend {
    /// Attempts to initialize a new Docker [`Backend`] with the default
    /// connection settings and the provided configuration for the backend.
    ///
    /// Note that, currently, we connect [using
    /// defaults](Docker::connect_with_defaults) when attempting to connect to
    /// the Docker daemon.
    pub fn initialize_default_with(config: Config) -> Result<Self> {
        let client = Docker::with_defaults()
            .context("error connecting to the Docker daemon—is it running?")?;

        Ok(Self { client, config })
    }

    /// Attempts to initialize a new Docker [`Backend`] with the default
    /// connection settings and default backend configuration.
    ///
    /// Note that, currently, we connect [using
    /// defaults](Docker::connect_with_defaults) when attempting to connect to
    /// the Docker daemon.
    pub fn initialize_default() -> Result<Self> {
        Self::initialize_default_with(Config::default())
    }
}

#[async_trait]
impl crate::Backend for Backend {
    fn default_name(&self) -> &'static str {
        "docker"
    }

    fn run(&self, task: Task) -> Result<BoxFuture<'static, Result<NonEmpty<Output>>>> {
        let client = self.client.clone();
        let cleanup = self.config.cleanup();
        let mounts = get_shared_mounts(task.shared_volumes())?;

        Ok(async move {
            let mut outputs = Vec::new();

            for execution in task.executions() {
                // (1) Create the container.
                let mut builder = client
                    .container_builder()
                    .image(execution.image())
                    .program(execution.program())
                    .args(execution.args())
                    .attach_stdout()
                    .attach_stderr()
                    .host_config(HostConfig {
                        mounts: Some(mounts.clone()),
                        ..task.resources().map(HostConfig::from).unwrap_or_default()
                    });

                if let Some(work_dir) = execution.work_dir() {
                    builder = builder.work_dir(work_dir.to_owned());
                }

                let container = Arc::new(
                    builder
                        .try_build(
                            &task
                                .name()
                                .context("task requires a name to run on the docker backend")?,
                        )
                        .await?,
                );

                // (2) Upload inputs to the container.
                //
                // TODO(clay): these could be cached.
                for task in task.inputs().cloned().map(|i| {
                    let container = container.clone();
                    tokio::spawn(async move {
                        let contents = i.fetch().await;
                        container.upload_file(i.path(), contents).await
                    })
                }) {
                    task.await??;
                }

                // (3) Start the container.
                let output = container.run().await.wrap_err("failed to run container")?;

                // (4) Cleanup the container (if desired).
                if cleanup {
                    container
                        .remove()
                        .await
                        .wrap_err("failed to remove container")?
                }

                outputs.push(output);
            }

            // SAFETY: each task _must_ have at least one execution, so at least one
            // execution result _must_ exist at this stage. Thus, this will always unwrap.
            Ok(NonEmpty::from_vec(outputs).unwrap())
        }
        .boxed())
    }
}

/// Gets the shared mounts (if any exist) from the shared volumes in a [`Task`]
/// (via [`Task::shared_volumes()`]).
fn get_shared_mounts<'a>(iter: impl Iterator<Item = &'a str>) -> Result<Vec<Mount>> {
    iter.map(|inner_path| {
        Ok(Mount {
            target: Some(inner_path.to_owned()),
            source: Some(
                TempDir::new()
                    .wrap_err("failed to create temporary directory")?
                    .into_path()
                    .to_str()
                    .with_context(|| "temporary path contains non-UTF-8 characters")?
                    .to_owned(),
            ),
            typ: Some(MountTypeEnum::BIND),
            read_only: Some(false),
            ..Default::default()
        })
    })
    .collect::<Result<Vec<_>>>()
}
