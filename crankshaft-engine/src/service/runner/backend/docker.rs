//! A Docker backend.

use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

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
use crankshaft_docker::EventOptions;
use crankshaft_docker::service::Service;
use crankshaft_events::Event;
use crankshaft_events::TaskId;
use crankshaft_events::next_task_id;
use crankshaft_events::send_event;
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
use crate::service::name::GeneratorIterator;
use crate::service::name::UniqueAlphanumeric;
use crate::task::Execution;
use crate::task::ExecutionResult;
use crate::task::Input;

impl From<crankshaft_docker::container::ExecutionResult> for ExecutionResult {
    fn from(execution_result: crankshaft_docker::container::ExecutionResult) -> Self {
        Self {
            image: Some(execution_result.image),
            status: execution_result.status,
        }
    }
}

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
            Self::Swarm(_) => true,
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
    /// The unique name generator for tasks without names.
    names: Arc<Mutex<GeneratorIterator<UniqueAlphanumeric>>>,
}

impl Backend {
    /// Attempts to initialize a new Docker [`Backend`] with the default
    /// connection settings and the provided configuration for the backend.
    ///
    /// Note that, currently, we connect [using defaults](Docker::with_defaults)
    /// when attempting to connect to the Docker daemon.
    pub async fn initialize_default_with(
        config: Config,
        names: Arc<Mutex<GeneratorIterator<UniqueAlphanumeric>>>,
    ) -> Result<Self> {
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
            names,
        })
    }

    /// Attempts to initialize a new Docker [`Backend`] with the default
    /// connection settings and default backend configuration.
    ///
    /// Note that, currently, we connect [using defaults](Docker::with_defaults)
    /// when attempting to connect to the Docker daemon.
    pub async fn initialize_default(
        names: Arc<Mutex<GeneratorIterator<UniqueAlphanumeric>>>,
    ) -> Result<Self> {
        Self::initialize_default_with(Config::default(), names).await
    }

    /// Runs an internal task without broadcasting events.
    ///
    /// Use this for backend housekeeping that was not requested by the user and
    /// must not expose a per-task cancellation token through
    /// [`Event::TaskCreated`]. The task can only be canceled through the
    /// `token` supplied by the caller.
    pub fn run_without_events(
        &self,
        task: Task,
        token: CancellationToken,
    ) -> Result<BoxFuture<'static, Result<NonEmpty<ExecutionResult>, TaskRunError>>> {
        <Self as crate::Backend>::run(self, task, None, token)
    }

    /// Gets a reference to the inner Docker client.
    pub fn client(&self) -> &Docker {
        &self.client
    }

    /// Gets information about the resources available to the Docker backend.
    pub fn resources(&self) -> &Resources {
        &self.resources
    }
}

/// Helper for cleaning up a container or service.
enum Cleanup {
    /// The cleanup is for a container.
    Container(Arc<Container>),
    /// The cleanup is for a service.
    Service(Arc<Service>),
}

impl Cleanup {
    /// Runs cleanup.
    async fn run(&self, canceled: bool) -> Result<()> {
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
            Self::Service(service) => service.delete().await.context("failed to delete service"),
        }
    }
}

/// Attempt to find a candidate image for the given execution.
async fn find_candidate_image(
    client: &Docker,
    execution: &Execution,
    token: CancellationToken,
    events: Option<broadcast::Sender<Event>>,
    task_id: TaskId,
) -> Result<String, TaskRunError> {
    let total_images = execution.images().len();
    let events = events.map(|e| (e, task_id));

    for (idx, try_image) in execution.images().iter().cloned().enumerate() {
        match client
            .ensure_image(&try_image, token.clone(), events.clone())
            .await
            .with_context(|| format!("failed to pull image `{try_image}`"))
        {
            Ok(Some(())) => {
                return Ok(try_image);
            }
            Ok(None) => return Err(TaskRunError::Canceled),
            Err(e) => {
                if idx == total_images - 1 {
                    return Err(TaskRunError::from(e));
                }

                continue;
            }
        }
    }

    unreachable!("there should always be at least one image available")
}

#[async_trait]
impl crate::Backend for Backend {
    fn default_name(&self) -> &'static str {
        "docker"
    }

    fn run(
        &self,
        task: Task,
        events: Option<broadcast::Sender<Event>>,
        token: CancellationToken,
    ) -> Result<BoxFuture<'static, Result<NonEmpty<ExecutionResult>, TaskRunError>>> {
        let task_id = next_task_id();
        let client = self.client.clone();
        let run_cleanup = self.config.cleanup();
        let events_config = self.config.events();
        let use_service = self.resources.use_service();
        let names = self.names.clone();

        let task_token = CancellationToken::new();

        Ok(async move {
            // Generate a name of the task if one wasn't provided
            let task_name = task.name.unwrap_or_else(|| {
                let mut generator = names.lock().unwrap();
                // SAFETY: the name generator should _never_ run out of entries.
                generator.next().unwrap()
            });

            let run = async {
                let tempdir = TempDir::new().context("failed to create temporary directory for mounts")?;

                let mut mounts = Vec::new();
                add_input_mounts(task.inputs, tempdir.path(), &mut mounts).await?;
                add_shared_mounts(task.volumes, tempdir.path(), &mut mounts)?;
                let mut outputs = Vec::new();

                for (i, execution) in task.executions.into_iter().enumerate() {
                    if token.is_cancelled() {
                        return Err(TaskRunError::Canceled);
                    }

                    // First, ensure the execution's image exists
                    let image = find_candidate_image(&client, &execution, token.clone(), events.clone(), task_id).await?;

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

                    let options = events.clone().map(|sender| EventOptions { sender, task_id, send_start: i == 0, user_config: events_config });
                    let attach_stdout = events.is_some() && events_config.send_stdout;
                    let attach_stderr = events.is_some() && events_config.send_stderr;

                    // Generate a name for the service or container
                    let name = {
                        let mut generator = names.lock().unwrap();
                        // SAFETY: the name generator should _never_ run out of entries.
                        generator.next().unwrap()
                    };

                    // Check to see if we should use the service API for running the task
                    let (result, cleanup) = if use_service {
                        let mut builder = client
                            .service_builder()
                            .name(&name)
                            .image(image)
                            .program(execution.program)
                            .args(execution.args)
                            .envs(execution.env)
                            .mounts(mounts.clone())
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

                        let service = Arc::new(builder.try_build().await.map_err(|e| TaskRunError::Other(e.into()))?);
                        info!("created service `{id}` (task `{task_name}`)", id = service.id());

                        select! {
                            // Always poll the cancellation token first
                            biased;
                            _= task_token.cancelled() => {
                                (Err(TaskRunError::Canceled), Cleanup::Service(service))
                            }
                            _ = token.cancelled() => {
                                (Err(TaskRunError::Canceled), Cleanup::Service(service))
                            }
                            res = service.run(&task_name, options) => {
                                (res.context("failed to run Docker service").map_err(TaskRunError::Other), Cleanup::Service(service))
                            }
                        }
                    } else {
                        let mut builder = client
                            .container_builder()
                            .name(&name)
                            .image(image)
                            .program(execution.program)
                            .args(execution.args)
                            .envs(execution.env)
                            .attach_stdout(attach_stdout)
                            .attach_stderr(attach_stderr)
                            .host_config(HostConfig {
                                mounts: Some(mounts.clone()),
                                // Ensure the caller's group id is added so that the container can access the mounts and working directory
                                #[cfg(unix)]
                                group_add: Some(vec![nix::unistd::Gid::effective().to_string()]),
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
                                .try_build()
                                .await.map_err(|e| TaskRunError::Other(e.into()))?,
                        );

                        info!("created container `{name}` (task `{task_name}`)", name = container.name());

                        select! {
                            // Always poll the cancellation token first
                            biased;
                            _ = task_token.cancelled() => {
                                (Err(TaskRunError::Canceled), Cleanup::Container(container))
                            }
                            _ = token.cancelled() => {
                                (Err(TaskRunError::Canceled), Cleanup::Container(container))
                            }
                            res = container.run(&task_name, options) => {
                                (res.context("failed to run Docker container").map_err(TaskRunError::Other), Cleanup::Container(container))
                            }
                        }
                    };

                    if run_cleanup {
                        let force_remove = matches!(result, Err(TaskRunError::Canceled));
                        cleanup.run(force_remove).await?;
                    }

                    outputs.push(result?);
                }

                // SAFETY: each task _must_ have at least one execution, so at least one
                // execution result _must_ exist at this stage. Thus, this will always unwrap.
                Ok(NonEmpty::from_vec(outputs).unwrap())
            };

            // Send the created event
            send_event!(events, Event::TaskCreated { id: task_id, name: task_name.clone(), tes_id: None, token: task_token.clone() });

            // Run the task to completion
            let result: Result<NonEmpty<ExecutionResult>, _> = run.await.map(|results| {
                // SAFETY: NonEmpty -> NonEmpty
                NonEmpty::collect(results.into_iter().map(Into::into)).unwrap()
            });

            // Send an event for the result
            match &result {
                Ok(results) => send_event!(
                    events,
                    Event::TaskCompleted {
                        id: task_id,
                        // SAFETY: NonEmpty -> NonEmpty
                        exit_statuses: NonEmpty::collect(results.iter().map(|r| r.status)).unwrap(),
                    }
                ),
                Err(TaskRunError::Canceled) => send_event!(
                    events,
                    Event::TaskCanceled {
                        id: task_id
                    }
                ),
                Err(TaskRunError::Preempted) => send_event!(
                    events,
                    Event::TaskPreempted {
                        id: task_id
                    }
                ),
                Err(TaskRunError::Other(e)) => send_event!(
                    events,
                    Event::TaskFailed {
                        id: task_id,
                        message: format!("{e:#}")
                    }
                ),
            }

            result
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

#[cfg(test)]
#[cfg(target_os = "linux")]
mod test {
    use std::assert_matches;
    use std::fs;

    use anyhow::Context;
    use futures::future::join_all;
    use nix::unistd::Gid;
    use tempfile::NamedTempFile;
    use url::Url;

    use super::*;
    use crate::service::runner::Backend as _;
    use crate::service::runner::NAME_BUFFER_LEN;
    use crate::task::Execution;
    use crate::task::Output;
    use crate::task::output::Type;

    async fn events(mut rx: broadcast::Receiver<Event>) -> Vec<Event> {
        let mut events = Vec::new();
        while let Ok(event) = rx.recv().await {
            events.push(event);
        }

        events
    }

    async fn create_backend(config: Config) -> Result<Backend> {
        let names = Arc::new(Mutex::new(GeneratorIterator::new(
            UniqueAlphanumeric::default_with_expected_generations(NAME_BUFFER_LEN),
            NAME_BUFFER_LEN,
        )));

        Backend::initialize_default_with(config, names)
            .await
            .context("failed to create backend")
    }

    #[tokio::test]
    async fn backend_adds_user_egid() -> anyhow::Result<()> {
        let backend = create_backend(Config::default()).await?;

        // Get the current user's effective gid
        let gid = Gid::effective();

        let stdout_path = NamedTempFile::new()
            .context("failed to create temporary file")?
            .into_temp_path();

        // Run the task
        let results = backend
            .run(
                Task::builder()
                    .executions(NonEmpty::new(
                        Execution::builder()
                            .images(["ubuntu:latest"])?
                            .program("/bin/sh")
                            .args([String::from("-c"), String::from("/usr/bin/id -G")])
                            .stdout("/mnt/stdout")
                            .build(),
                    ))
                    .outputs(vec![
                        Output::builder()
                            .ty(Type::File)
                            .path("/mnt/stdout")
                            .url(
                                Url::from_file_path(&stdout_path)
                                    .expect("failed to get URL for stdout path"),
                            )
                            .build(),
                    ])
                    .build(),
                None,
                CancellationToken::new(),
            )
            .context("failed to run task")?
            .await
            .context("task execution failed")?;

        assert!(results.first().status.success(), "container failed");

        // Assert that the command output had the user's group added
        let stdout = fs::read_to_string(&stdout_path).context("failed to read stdout file")?;
        assert!(
            stdout.contains(&gid.to_string()),
            "task stdout of `{stdout}` did not contain the expected output"
        );

        Ok(())
    }

    #[tokio::test]
    async fn backend_supports_fallback_images() -> anyhow::Result<()> {
        let backend = create_backend(Config::default()).await?;

        let stdout_path = NamedTempFile::new()
            .context("failed to create temporary file")?
            .into_temp_path();

        let (events_tx, events_rx) = broadcast::channel(1024);
        let events = tokio::task::spawn(events(events_rx));

        let results = backend
            .run(
                Task::builder()
                    .executions(NonEmpty::new(
                        Execution::builder()
                            .images([
                                "ubuntu:super_fake_tag_that_doesnt_exist",
                                "ubuntu:this_tag_is_even_more_fake",
                                "ubuntu:latest",
                            ])?
                            .program("/bin/sh")
                            .args([
                                String::from("-c"),
                                String::from("/usr/bin/echo \"Hello, world!\""),
                            ])
                            .stdout("/mnt/stdout")
                            .build(),
                    ))
                    .outputs(vec![
                        Output::builder()
                            .ty(Type::File)
                            .path("/mnt/stdout")
                            .url(
                                Url::from_file_path(&stdout_path)
                                    .expect("failed to get URL for stdout path"),
                            )
                            .build(),
                    ])
                    .build(),
                Some(events_tx),
                CancellationToken::new(),
            )
            .context("failed to run task")?
            .await
            .context("task execution failed")?;

        let events = events.await.unwrap();

        assert!(results.first().status.success(), "container failed");
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, Event::ImagePullFailed { .. }))
                .count(),
            2
        );

        // Assert that the command was run
        let stdout = fs::read_to_string(&stdout_path).context("failed to read stdout file")?;
        assert!(
            stdout.contains("Hello, world!"),
            "task stdout of `{stdout}` did not contain the expected output"
        );

        Ok(())
    }

    #[tokio::test]
    async fn concurrent_task_events() -> anyhow::Result<()> {
        fn assert_events(events: &[Event], stdout: &[u8]) -> TaskId {
            // There should be six or eight events generated
            // Eight events indicates the image was pulled
            assert!(events.len() == 6 || events.len() == 8);

            // The first event should be the created event; extract the id
            let task_id = match &events[0] {
                Event::TaskCreated { id, .. } => *id,
                _ => panic!("the first event should be the created event"),
            };

            if events.len() == 6 {
                assert_matches!(&events[1], Event::TaskContainerCreated { id, .. } if *id == task_id);
                assert_matches!(&events[2], Event::TaskStarted { id } if *id == task_id);
                assert_matches!(&events[3], Event::TaskStdout { id, message } if *id == task_id && message == stdout);
                assert_matches!(&events[4], Event::TaskContainerExited { id, exit_status, .. } if *id == task_id && exit_status.success());
                assert_matches!(&events[5], Event::TaskCompleted { id, exit_statuses } if *id == task_id && exit_statuses[0].success());
            } else if events.len() == 8 {
                assert_matches!(&events[1], Event::ImagePullStarted { id, name } if *id == task_id && name == "ubuntu:latest");
                assert_matches!(&events[2], Event::ImagePullFinished { id, name } if *id == task_id && name == "ubuntu:latest");
                assert_matches!(&events[3], Event::TaskContainerCreated { id, .. } if *id == task_id);
                assert_matches!(&events[4], Event::TaskStarted { id } if *id == task_id);
                assert_matches!(&events[5], Event::TaskStdout { id, message } if *id == task_id && message == stdout);
                assert_matches!(&events[6], Event::TaskContainerExited { id, exit_status, .. } if *id == task_id && exit_status.success());
                assert_matches!(&events[7], Event::TaskCompleted { id, exit_statuses } if *id == task_id && exit_statuses[0].success());
            } else {
                panic!("unexpected number of events");
            }

            task_id
        }

        let backend = Arc::new(create_backend(Config::default()).await?);

        let (events1_tx, events1_rx) = broadcast::channel(1024);
        let events1 = tokio::task::spawn(events(events1_rx));

        let (events2_tx, events2_rx) = broadcast::channel(1024);
        let events2 = tokio::task::spawn(events(events2_rx));

        // Spawn the first task
        let backend1 = backend.clone();
        let task1 = tokio::spawn(async move {
            backend1
                .run(
                    Task::builder()
                        .executions(NonEmpty::new(
                            Execution::builder()
                                .images(["ubuntu:latest"])?
                                .program("/bin/sh")
                                .args([String::from("-c"), String::from("echo task1")])
                                .build(),
                        ))
                        .build(),
                    Some(events1_tx),
                    CancellationToken::new(),
                )
                .expect("failed to run task")
                .await?;

            anyhow::Ok(())
        });

        // Spawn the second task
        let task2 = tokio::spawn(async move {
            backend
                .run(
                    Task::builder()
                        .executions(NonEmpty::new(
                            Execution::builder()
                                .images(["ubuntu:latest"])?
                                .program("/bin/sh")
                                .args([String::from("-c"), String::from("echo task2")])
                                .build(),
                        ))
                        .build(),
                    Some(events2_tx),
                    CancellationToken::new(),
                )
                .expect("failed to run task")
                .await?;

            anyhow::Ok(())
        });

        // Wait for the tasks to complete and check for errors
        for result in join_all([task1, task2]).await {
            result
                .context("failed to join task")?
                .context("task failed")?;
        }

        let events1 = events1
            .await
            .context("failed to wait for the first task's events")?;
        let events2 = events2
            .await
            .context("failed to wait for the first task's events")?;

        let task_id1 = assert_events(&events1, b"task1\n");
        let task_id2 = assert_events(&events2, b"task2\n");
        assert!(task_id1 != task_id2, "expected different task identifiers");

        Ok(())
    }

    fn internal_task() -> Result<Task> {
        Ok(Task::builder()
            .executions(NonEmpty::new(
                Execution::builder()
                    .images(["ubuntu:latest"])?
                    .program("/bin/true")
                    .build(),
            ))
            .build())
    }

    #[tokio::test]
    async fn backend_run_without_events_completes() -> anyhow::Result<()> {
        let backend = create_backend(Config::default()).await?;

        let results = backend
            .run_without_events(internal_task()?, CancellationToken::new())
            .context("failed to run internal task")?
            .await
            .context("internal task execution failed")?;

        assert!(
            results.first().status.success(),
            "internal container failed"
        );

        Ok(())
    }

    #[tokio::test]
    async fn backend_run_without_events_honors_caller_token() -> anyhow::Result<()> {
        let backend = create_backend(Config::default()).await?;
        let token = CancellationToken::new();
        token.cancel();

        let result = backend
            .run_without_events(internal_task()?, token)
            .context("failed to run internal task")?
            .await;

        assert_matches!(result, Err(TaskRunError::Canceled));

        Ok(())
    }
}
