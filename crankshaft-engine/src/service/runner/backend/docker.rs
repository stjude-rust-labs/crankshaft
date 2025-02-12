//! A Docker backend.

use std::process::Output;
use std::sync::Arc;

use async_trait::async_trait;
use bollard::secret::HostConfig;
use bollard::secret::LocalNodeState;
use bollard::secret::Mount;
use bollard::secret::MountTypeEnum;
use bollard::secret::NodeSpecAvailabilityEnum;
use bollard::secret::NodeState;
use crankshaft_config::backend::docker::Config;
use crankshaft_docker::Container;
use crankshaft_docker::Docker;
use crankshaft_docker::service::Service;
use eyre::Context;
use eyre::ContextCompat;
use eyre::bail;
use eyre::eyre;
use futures::FutureExt;
use futures::future::BoxFuture;
use nonempty::NonEmpty;
use tempfile::TempDir;
use tokio::select;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::info;

use crate::Result;
use crate::Task;

/// Represents resource information abouta Docker swarm.
#[derive(Debug, Default, Clone, Copy)]
pub struct SwarmResources {
    /// The number of nodes in the swarm.
    pub nodes: usize,
    /// The total CPUs available to the swarm.
    pub cpu: u64,
    /// The total memory of the swarm, in bytes.
    pub memory: u64,
    /// The maximum CPUs for any of the nodes in the swarm.
    pub max_cpu: u64,
    /// The maximum memory for any of the nodes in the swarm.
    pub max_memory: u64,
}

/// Represents resource information about a local Docker daemon.
#[derive(Debug, Default, Clone, Copy)]
pub struct LocalResources {
    /// The total CPUs available to the local Docker daemon.
    pub cpu: u64,
    /// The total memory available to the local Docker daemon, in bytes.
    pub memory: u64,
}

/// Represents information about Docker's available resources.
#[derive(Debug, Clone, Copy)]
pub enum Resources {
    /// The resources are for a local Docker daemon.
    Local(LocalResources),
    /// The resources are for a Docker swarm.
    Swarm(SwarmResources),
}

impl Resources {
    /// Determines if the docker backend will use a service instead of a
    /// container based on the resources available.
    ///
    /// A service should only be used when Docker is in a swarm with more than
    /// one node. This allows for the Swarm manager to schedule the container.
    ///
    /// Otherwise, we'll use a single local container.
    pub fn use_service(&self) -> bool {
        match self {
            Self::Local(_) => false,
            Self::Swarm(swarm) => swarm.nodes >= 2,
        }
    }
}

/// A local execution backend.
#[derive(Debug)]
pub struct Backend {
    /// A handle to the inner docker client.
    client: Docker,
    /// Configuration for the backend.
    config: Config,
    /// The available resources reported by Docker.
    resources: Resources,
}

impl Backend {
    /// Attempts to initialize a new Docker [`Backend`] with the default
    /// connection settings and the provided configuration for the backend.
    ///
    /// Note that, currently, we connect [using
    /// defaults](Docker::connect_with_defaults) when attempting to connect to
    /// the Docker daemon.
    pub async fn initialize_default_with(config: Config) -> Result<Self> {
        let client =
            Docker::with_defaults().context("failed to connect to the local Docker daemon")?;

        let info = client
            .info()
            .await
            .context("failed to retrieve local Docker daemon information")?;

        // Check to see if the daemon is part of an active swarm or not
        // If the daemon is part of a swarm, but the node is not active or a manager, we
        // can't spawn tasks
        let swarm = if let Some(swarm) = &info.swarm {
            match (&swarm.node_id, swarm.local_node_state) {
                (Some(id), Some(LocalNodeState::ACTIVE)) if !id.is_empty() => {
                    // Part of an active swarm, check to see if the node is a manager
                    // Default is false as documented here: https://docs.docker.com/reference/api/engine/version/v1.47/#tag/System/operation/SystemInfo
                    if !swarm.control_available.unwrap_or(false) {
                        bail!(
                            "the local Docker daemon is part of a swarm but cannot be used to \
                             create tasks (the node is not a manager)"
                        );
                    }

                    // Only look at active and ready nodes in the swarm that are reporting their
                    // resources
                    let nodes = client
                        .nodes()
                        .await
                        .context("failed to retrieve Docker swarm node list")?;
                    let mut swarm = SwarmResources::default();
                    for node in nodes.iter().filter(|n| {
                        n.description
                            .as_ref()
                            .map(|d| d.resources.is_some())
                            .unwrap_or(false)
                            && n.spec
                                .as_ref()
                                .map(|s| s.availability == Some(NodeSpecAvailabilityEnum::ACTIVE))
                                .unwrap_or(false)
                            && n.status
                                .as_ref()
                                .map(|s| s.state == Some(NodeState::READY))
                                .unwrap_or(false)
                    }) {
                        swarm.nodes += 1;

                        let resources = node
                            .description
                            .as_ref()
                            .unwrap()
                            .resources
                            .as_ref()
                            .unwrap();

                        let node_cpu: u64 = resources
                            .nano_cpus
                            .map(|n| n / 1_000_000_000)
                            .context("Docker daemon reported an active node with no CPUs")?
                            .try_into()
                            .context("node CPU count is negative")?;
                        swarm.cpu += node_cpu;
                        swarm.max_cpu = swarm.max_cpu.max(node_cpu);

                        let node_memory: u64 = resources
                            .memory_bytes
                            .context("Docker daemon reported an active node with no memory")?
                            .try_into()
                            .context("node memory is negative")?;
                        swarm.memory += node_memory;
                        swarm.max_memory = swarm.max_memory.max(node_memory);

                        debug!(
                            id = node
                                .id
                                .as_ref()
                                .context("Docker daemon reported a node without an identifier")?,
                            total_cpu = node_cpu,
                            total_memory = node_memory,
                            "found Docker swarm node"
                        );
                    }

                    if swarm.nodes == 0 {
                        bail!(
                            "the local Docker daemon is part of a swarm but there are no active \
                             and ready nodes"
                        );
                    }

                    Some(swarm)
                }
                (Some(id), _) if !id.is_empty() => {
                    bail!(
                        "the local Docker daemon is part of a swarm but the node state is not \
                         active"
                    );
                }
                _ => {
                    // Not part of a swarm
                    None
                }
            }
        } else {
            None
        };

        let resources = match swarm {
            Some(swarm) => {
                info!(
                    nodes = swarm.nodes,
                    cpu = swarm.cpu,
                    memory = swarm.memory,
                    max_cpu = swarm.max_cpu,
                    max_memory = swarm.max_memory,
                    "Docker backend is interacting with a swarm"
                );

                Resources::Swarm(swarm)
            }
            None => {
                let cpu = info
                    .ncpu
                    .map(|n| {
                        n.try_into()
                            .context("Docker daemon reported a negative CPU count")
                    })
                    .transpose()?
                    .context("Docker daemon did not report a CPU count")?;
                let memory = info
                    .mem_total
                    .map(|n| {
                        n.try_into()
                            .context("Docker daemon reported a negative total memory")
                    })
                    .transpose()?
                    .context("Docker daemon did not report a memory total")?;
                info!(
                    cpu,
                    memory, "Docker backend is interacting with a local Docker daemon"
                );

                Resources::Local(LocalResources { cpu, memory })
            }
        };

        Ok(Self {
            client,
            config,
            resources,
        })
    }

    /// Attempts to initialize a new Docker [`Backend`] with the default
    /// connection settings and default backend configuration.
    ///
    /// Note that, currently, we connect [using
    /// defaults](Docker::connect_with_defaults) when attempting to connect to
    /// the Docker daemon.
    pub async fn initialize_default() -> Result<Self> {
        Self::initialize_default_with(Config::default()).await
    }

    /// Gets information about the resources available to the Docker backend.
    pub fn resources(&self) -> &Resources {
        &self.resources
    }

    /// Runs a task using a container.
    async fn run_with_container(
        task: &Task,
        execution_index: usize,
        started: Arc<dyn Fn(usize) + Send + Sync + 'static>,
        container: Arc<Container>,
    ) -> Result<Output> {
        // TODO(clay): these could be cached.
        for task in task.inputs().map(|i| {
            let container = container.clone();
            tokio::spawn(async move {
                let contents = i.fetch().await;
                container.upload_file(i.path(), &contents).await
            })
        }) {
            task.await??;
        }

        container
            .run(|| started(execution_index))
            .await
            .wrap_err("failed to run Docker container")
    }

    /// Runs a task using a Docker service.
    async fn run_with_service(
        execution_index: usize,
        started: Arc<dyn Fn(usize) + Send + Sync + 'static>,
        service: Arc<Service>,
    ) -> Result<Output> {
        // TODO: handle inputs
        service
            .run(|| started(execution_index))
            .await
            .wrap_err("failed to run Docker service")
    }
}

#[async_trait]
impl crate::Backend for Backend {
    fn default_name(&self) -> &'static str {
        "docker"
    }

    fn run(
        &self,
        task: Task,
        started: Arc<dyn Fn(usize) + Send + Sync + 'static>,
        token: CancellationToken,
    ) -> Result<BoxFuture<'static, Result<NonEmpty<Output>>>> {
        // Helper for cleanup
        enum Cleaner {
            /// The cleanup is for a container.
            Container(Arc<Container>),
            /// The cleanup is for a service.
            Service(Arc<Service>),
        }

        impl Cleaner {
            /// Runs cleanup.
            async fn cleanup(&self, cancelled: bool) -> Result<()> {
                match self {
                    Self::Container(container) => {
                        if cancelled {
                            container
                                .force_remove()
                                .await
                                .wrap_err("failed to force remove container")
                        } else {
                            container
                                .remove()
                                .await
                                .wrap_err("failed to remove container")
                        }
                    }
                    Self::Service(service) => {
                        service.delete().await.wrap_err("failed to delete service")
                    }
                }
            }
        }

        let client = self.client.clone();
        let cleanup = self.config.cleanup();
        let resources = self.resources;
        let mounts = get_shared_mounts(task.shared_volumes())?;

        Ok(async move {
            // Check to see if we should use the service API for running the task
            let mut outputs = Vec::new();

            let name = task
                    .name()
                    .context("task requires a name to run on the Docker backend")?
                    .to_owned();

            for (execution_index, execution) in task.executions().enumerate() {
                if token.is_cancelled() {
                    bail!("task has been cancelled");
                }

                let (result, cleaner) = if resources.use_service() {
                    let mut builder = client
                        .service_builder()
                        .image(execution.image())
                        .program(execution.program())
                        .args(execution.args())
                        .attach_stdout()
                        .attach_stderr();

                    if let Some(work_dir) = execution.work_dir() {
                        builder = builder.work_dir(work_dir);
                    }

                    let service = Arc::new(builder.try_build(&name).await?);

                    select! {
                        _ = token.cancelled() => {
                            (Err(eyre!("task has been cancelled")), Cleaner::Service(service))
                        }
                        res = Self::run_with_service(execution_index, started.clone(), service.clone()) => {
                            (res, Cleaner::Service(service))
                        }
                    }
                } else {
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
                        builder = builder.work_dir(work_dir);
                    }

                    let container = Arc::new(
                        builder
                            .try_build(&name)
                            .await?,
                    );

                    select! {
                        _ = token.cancelled() => {
                            (Err(eyre!("task has been cancelled")), Cleaner::Container(container))
                        }
                        res = Self::run_with_container(&task, execution_index, started.clone(), container.clone()) => {
                            (res, Cleaner::Container(container))
                        }
                    }
                };

                if cleanup {
                    cleaner.cleanup(token.is_cancelled()).await?;
                }

                outputs.push(result?);
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
