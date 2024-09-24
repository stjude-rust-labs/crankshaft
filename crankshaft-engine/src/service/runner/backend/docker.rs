//! A Docker backend.

use async_trait::async_trait;
use bollard::secret::HostConfig;
use bollard::secret::Mount;
use bollard::secret::MountTypeEnum;
use crankshaft_config::backend::docker::Config;
use crankshaft_docker::Docker;
use eyre::Context;
use futures::future::BoxFuture;
use futures::stream::FuturesUnordered;
use futures::FutureExt;
use futures::StreamExt;
use nonempty::NonEmpty;
use tempfile::TempDir;

use crate::service::runner::backend::TaskResult;
use crate::Result;
use crate::Task;

/// The working dir name inside the docker container
pub const WORKDIR: &str = "/workdir";

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

    fn run(&self, task: Task) -> BoxFuture<'static, TaskResult> {
        run(self, task)
    }
}

/// Gets the shared mounts (if any exist) from the shared volumes in a [`Task`]
/// (via [`Task::shared_volumes()`]).
fn get_shared_mounts<'a>(volumes: Option<impl Iterator<Item = &'a str>>) -> Option<Vec<Mount>> {
    volumes.map(|iter| {
        iter.map(|inner_path| {
            Mount {
                target: Some(inner_path.to_owned()),
                source: Some(
                    TempDir::new()
                        // SAFETY: for now, this is essentially a workaround to the fact
                        // that we do not return a [`Result`] in the `run()` method. It's
                        // certainly possible for this to fail, but I feel it's unlikely
                        // enough to occur in early development that handling this properly
                        // can be elided for now.
                        //
                        // TODO(clay): more properly handle this later.
                        .expect("could not initialize tempdir")
                        // NOTE: this is *required* because it causes the
                        // temporary directory to no longer be dropped when the
                        // [`TempDir`] goes out of scope. In other words, simply
                        // referring to [`path()`] isn't sufficient (even though
                        // it would suit our purposes from the perspective of
                        // getting a [`str`] representation).
                        .into_path()
                        .to_str()
                        // SAFETY: essentially the above reasoning—it's unlikely
                        // that this will fail in early testing, but we should
                        // come back to more properly handling this later.
                        //
                        // TODO(clay): more properly handle this later.
                        .unwrap()
                        .to_owned(),
                ),
                typ: Some(MountTypeEnum::BIND),
                read_only: Some(false),
                ..Default::default()
            }
        })
        .collect::<Vec<_>>()
    })
}

/// Runs a task using the Docker backend.
fn run(backend: &Backend, task: Task) -> BoxFuture<'static, TaskResult> {
    let client = backend.client.clone();
    let cleanup = backend.config.cleanup();
    let mounts = get_shared_mounts(task.shared_volumes());

    async move {
        let mut outputs = Vec::new();

        for execution in task.executions() {
            // (1) Create the container.
            let mut builder = client
                .container_builder()
                .image(execution.image())
                .command(
                    execution
                        .args()
                        .into_iter()
                        .map(|s| s.to_owned())
                        .collect::<Vec<_>>(),
                )
                .attached(true)
                .host_config(HostConfig {
                    mounts: mounts.clone(),
                    ..task.resources().map(HostConfig::from).unwrap_or_default()
                });

            if let Some(workdir) = execution.workdir() {
                builder = builder.workdir(workdir.to_owned());
            }

            let container = builder.try_create(&task.name().unwrap()).await.unwrap();

            // (2) Upload inputs to the container.
            //
            // TODO(clay): these could be cached.
            if let Some(inputs) = task.inputs() {
                let futures = inputs
                    .map(|input| async {
                        let contents = input.fetch().await;
                        container.upload_file(input.path(), contents).await
                    })
                    .collect::<FuturesUnordered<_>>();

                // NOTE: this is just an unfancy way to evaluate all of the
                // above futures.
                //
                // TODO(clay): make this more elegant.
                futures.for_each(|_| async {}).await;
            };

            // (3) Start the container.
            let output = container.run().await.unwrap();

            // (4) Cleanup the container (if desired).
            if cleanup {
                container
                    .remove()
                    .await
                    // SAFETY: this should always unwrap for now, but we should
                    // revisit this in the future to more elegantly handle the
                    // situation.
                    //
                    // TODO(clay): more elegantly handle this situation.
                    .unwrap();
            }

            outputs.push(output);
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
