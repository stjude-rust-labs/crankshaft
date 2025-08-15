//! A Docker backend.

use std::path::Path;
use std::path::PathBuf;
use std::process::ExitStatus;
use std::sync::Arc;

use anyhow::Context;
use anyhow::Result;
use anyhow::anyhow;
use anyhow::bail;
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
use crankshaft_monitor::proto::Event;
use crankshaft_monitor::proto::EventType;
use crankshaft_monitor::proto::Resources as ProtoResources;
use crankshaft_monitor::proto::event::Payload;
use crankshaft_monitor::send_event;
use futures::FutureExt;
use futures::future::BoxFuture;
use nonempty::NonEmpty;
use tempfile::TempDir;
use tokio::select;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::info;

use super::TaskRunError;
use crate::Task;
use crate::task::Input;

/// Represents resource information about a Docker swarm.
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

impl From<SwarmResources> for ProtoResources {
    fn from(value: SwarmResources) -> Self {
        Self {
            nodes: value.nodes as f64,
            cpu: value.cpu as f64,
            memory: value.memory as f64,
            max_cpu: value.max_cpu as f64,
            max_memory: value.max_memory as f64,
        }
    }
}

impl From<LocalResources> for ProtoResources {
    fn from(value: LocalResources) -> Self {
        Self {
            cpu: value.cpu as f64,
            memory: value.memory as f64,
            nodes: 1.0,
            max_cpu: value.cpu as f64,
            max_memory: value.memory as f64,
        }
    }
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
    /// Gets the number of nodes.
    pub fn nodes(&self) -> usize {
        match self {
            Self::Local(_) => 1,
            Self::Swarm(resources) => resources.nodes,
        }
    }

    /// Gets the total CPUs available.
    pub fn cpu(&self) -> u64 {
        match self {
            Self::Local(resources) => resources.cpu,
            Self::Swarm(resources) => resources.cpu,
        }
    }

    /// Gets the total memory available, in bytes.
    pub fn memory(&self) -> u64 {
        match self {
            Self::Local(resources) => resources.memory,
            Self::Swarm(resources) => resources.memory,
        }
    }

    /// Gets the maximum CPUs available for a single node.
    pub fn max_cpu(&self) -> u64 {
        match self {
            Self::Local(resources) => resources.cpu,
            Self::Swarm(resources) => resources.max_cpu,
        }
    }

    /// Gets the maximum memory available for a single node, in bytes.
    pub fn max_memory(&self) -> u64 {
        match self {
            Self::Local(resources) => resources.memory,
            Self::Swarm(resources) => resources.max_memory,
        }
    }

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

    /// Helper method to convert local resource type to proto `Resource`
    pub fn to_proto(&self) -> Option<Payload> {
        match self {
            Resources::Swarm(r) => Some(Payload::Resources((*r).into())),
            Resources::Local(r) => Some(Payload::Resources((*r).into())),
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
    /// Note that, currently, we connect [using defaults](Docker::with_defaults)
    /// when attempting to connect to the Docker daemon.
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
    /// Note that, currently, we connect [using defaults](Docker::with_defaults)
    /// when attempting to connect to the Docker daemon.
    pub async fn initialize_default() -> Result<Self> {
        Self::initialize_default_with(Config::default()).await
    }

    /// Gets information about the resources available to the Docker backend.
    pub fn resources(&self) -> &Resources {
        &self.resources
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
        event_sender: Option<broadcast::Sender<Event>>,
        token: CancellationToken,
    ) -> Result<BoxFuture<'static, Result<NonEmpty<ExitStatus>, TaskRunError>>> {
        // Helper for cleanup
        enum Cleaner {
            /// The cleanup is for a container.
            Container(Arc<Container>),
            /// The cleanup is for a service.
            Service(Arc<Service>),
        }

        impl Cleaner {
            /// Runs cleanup.
            async fn cleanup(&self, canceled: bool) -> Result<()> {
                match self {
                    Self::Container(container) => {
                        if canceled {
                            container
                                .force_remove()
                                .await
                                .context("failed to force remove container")
                        } else {
                            container
                                .remove()
                                .await
                                .context("failed to remove container")
                        }
                    }
                    Self::Service(service) => {
                        service.delete().await.context("failed to delete service")
                    }
                }
            }
        }

        let client = self.client.clone();
        let cleanup = self.config.cleanup();
        let resources = self.resources;

        Ok(async move {
            let tempdir = TempDir::new().context("failed to create temporary directory for mounts")?;

            let mut mounts = Vec::new();
            add_input_mounts(task.inputs, tempdir.path(), &mut mounts).await?;
            add_shared_mounts(task.volumes, tempdir.path(), &mut mounts)?;
            let mut outputs = Vec::new();

            let name = task
                    .name
                    .context("task requires a name to run on the Docker backend")?;

            for execution in task.executions {
                if token.is_cancelled() {
                    return Err(TaskRunError::Canceled);
                }

                // First ensure the execution's image exists
                client
                    .ensure_image(&execution.image)
                    .await
                    .with_context(|| format!("failed to pull image `{image}`", image = execution.image))?;

                // Look for the path where the caller wants stdout saved to
                let stdout = execution.stdout.as_ref().and_then(|p| {
                    let url = task.outputs.iter().find_map(|o| if o.path == *p {
                        Some(&o.url)
                    } else {
                        None
                    })?;

                    match url.scheme() {
                        "file" => {
                            Some(url.to_file_path().map_err(|_| {
                                anyhow!(
                                    "stdout URL `{url}` has a file scheme but cannot be represented as a file path"
                                )
                            }))
                        }
                        _ => Some(Err(anyhow!("unsupported scheme for stdout URL `{url}`")))
                    }

                }).transpose()?;

                // Look for the path where the caller wants stderr saved to
                let stderr = execution.stderr.as_ref().and_then(|p| {
                    let url = task.outputs.iter().find_map(|o| if o.path == *p {
                        Some(&o.url)
                    } else {
                        None
                    })?;

                    match url.scheme() {
                        "file" => {
                            Some(url.to_file_path().map_err(|_| {
                                anyhow!(
                                    "stderr URL `{url}` has a file scheme but cannot be represented as a file path"
                                )
                            }))
                        }
                        _ => Some(Err(anyhow!("unsupported scheme for stderr URL `{url}`")))
                    }

                }).transpose()?;

                // Check to see if we should use the service API for running the task
                let (result, cleaner) = if resources.use_service() {
                    let mut builder = client
                        .service_builder()
                        .image(execution.image)
                        .program(execution.program)
                        .args(execution.args)
                        .envs(execution.env)
                        .resources(task.resources.as_ref().map(Into::into).unwrap_or_default());

                    if let Some(stdout) = stdout {
                        builder = builder.stdout(stdout);
                    }

                    if let Some(stderr) = stderr {
                        builder = builder.stderr(stderr);
                    }

                    if let Some(work_dir) = execution.work_dir {
                        builder = builder.work_dir(work_dir);
                    }

                    let service = Arc::new(builder.try_build(&name).await.map_err(|e| TaskRunError::Other(e.into()))?);


                    if let Some(Payload::Resources(payload)) = resources.to_proto() {
                        send_event!(&event_sender, "Docker-swarm".to_string(), EventType::ServiceStarted, resource = payload);
                    }

                    select! {
                        // Always poll the cancellation token first
                        biased;

                        _ = token.cancelled() => {
                            (Err(TaskRunError::Canceled), Cleaner::Service(service))
                        }
                        res = service.run(&name,event_sender.clone()) => {
                            (res.context("failed to run Docker service").map_err(TaskRunError::Other), Cleaner::Service(service))
                        }
                    }
                } else {
                   let mut builder = client
                        .container_builder()
                        .image(execution.image)
                        .program(execution.program)
                        .args(execution.args)
                        .envs(execution.env)
                        .host_config(HostConfig {
                            mounts: Some(mounts.clone()),
                            ..task.resources.as_ref().map(|r| r.into()).unwrap_or_default()
                        });

                    if let Some(stdout) = stdout {
                        builder = builder.stdout(stdout);
                    }

                    if let Some(stderr) = stderr {
                        builder = builder.stderr(stderr);
                    }

                    if let Some(work_dir) = execution.work_dir {
                        builder = builder.work_dir(work_dir);
                    }

                    let container = Arc::new(
                        builder
                            .try_build(name.clone())
                            .await.map_err(|e| TaskRunError::Other(e.into()))?,
                    );

                    if let Some(Payload::Resources(payload)) = resources.to_proto() {
                        send_event!(&event_sender, "Docker-container".to_string() , EventType::ContainerStarted, resource = payload);
                    }


                    select! {
                        // Always poll the cancellation token first
                        biased;

                        _ = token.cancelled() => {
                            (Err(TaskRunError::Canceled), Cleaner::Container(container))
                        }
                        res = container.run(&name,event_sender.clone()) => {
                            (res.context("failed to run Docker container").map_err(TaskRunError::Other), Cleaner::Container(container))
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

/// Adds input mounts to the list of mounts.
///
/// Bind mounts are created for any input specified as a path.
///
/// For inputs not specified by a path, the contents are fetched and written to
/// a file within the provided temporary directory.
///
/// Errors may be returned if an input's contents could not be fetched.
async fn add_input_mounts(
    inputs: Vec<Input>,
    temp_dir: &Path,
    mounts: &mut Vec<Mount>,
) -> Result<()> {
    for input in inputs {
        let target = input.path;
        let source = input.contents.fetch(temp_dir).await?;

        mounts.push(Mount {
            target: Some(target),
            source: Some(
                source
                    .to_str()
                    .with_context(|| {
                        format!("path `{source}` is not UTF-8", source = source.display())
                    })?
                    .to_string(),
            ),
            typ: Some(MountTypeEnum::BIND),
            read_only: Some(input.read_only),
            ..Default::default()
        });
    }

    Ok(())
}

/// Gets the shared mounts (if any exist) from the shared volumes in a [`Task`]
/// (via [`Task::shared_volumes()`]).
fn add_shared_mounts(volumes: Vec<String>, tempdir: &Path, mounts: &mut Vec<Mount>) -> Result<()> {
    for volume in volumes {
        // Create new temporary directory in the provided temporary directory
        // The call to `into_path` will prevent the directory from being deleted on
        // drop; instead, we're relying on the parent temporary directory to delete it
        let path = TempDir::new_in(tempdir)
            .with_context(|| {
                format!(
                    "failed to create temporary directory in `{tempdir}`",
                    tempdir = tempdir.display()
                )
            })?
            .keep()
            .into_os_string()
            .into_string()
            .map_err(|path| {
                anyhow!(
                    "temporary directory path `{path}` is not UTF-8",
                    path = PathBuf::from(&path).display()
                )
            })?;

        mounts.push(Mount {
            target: Some(volume),
            source: Some(path),
            typ: Some(MountTypeEnum::BIND),
            read_only: Some(false),
            ..Default::default()
        });
    }

    Ok(())
}
