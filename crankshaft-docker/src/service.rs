//! Services.

use std::collections::HashMap;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt as _;
#[cfg(windows)]
use std::os::windows::process::ExitStatusExt as _;
use std::path::PathBuf;
use std::process::ExitStatus;
use std::time::Duration;

use bollard::Docker;
use bollard::container::LogOutput;
use bollard::query_parameters::InspectContainerOptions;
use bollard::query_parameters::ListTasksOptions;
use bollard::query_parameters::LogsOptions;
use bollard::query_parameters::WaitContainerOptions;
use bollard::secret::ContainerWaitResponse;
use bollard::secret::TaskState;

mod builder;

pub use builder::Builder;
use futures::StreamExt;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::time::sleep;
use tracing::debug;
use tracing::info;
use tracing::trace;

use crate::Error;
use crate::Result;

/// A docker service.
///
/// Docker services are used to run tasks when Docker is configured to use a
/// swarm.
///
/// This allows the swarm manager to schedule the task on available resources.
///
/// The service will always have a single replica of the task and the task will
/// not restart.
pub struct Service {
    /// A reference to the [`Docker`] client that will be used to create this
    /// service.
    client: Docker,

    /// The name of the service.
    id: String,

    /// The path to the file to write the container's stdout stream to.
    stdout: Option<PathBuf>,

    /// The path to the file to write the container's stderr stream to.
    stderr: Option<PathBuf>,
}

impl Service {
    /// Creates a new [`Service`] if you already know the id of the service.
    ///
    /// You should typically use a [`Builder`] unless you receive the service
    /// name externally from a user (say, on the command line as an argument).
    pub fn new(
        client: Docker,
        id: String,
        stdout: Option<PathBuf>,
        stderr: Option<PathBuf>,
    ) -> Self {
        Self {
            client,
            id,
            stdout,
            stderr,
        }
    }

    /// Runs a service and waits for the task execution to end.
    pub async fn run(&self, name: &str, started: impl FnOnce()) -> Result<ExitStatus> {
        let (container_id, exit_code) = loop {
            trace!(
                "polling tasks for service `{id}` (task `{name}`)",
                id = self.id
            );

            // Get the list of tasks for the service (there should be only one)
            let tasks = self
                .client
                .list_tasks(Some(ListTasksOptions {
                    filters: Some(HashMap::from_iter([(
                        String::from("service"),
                        vec![self.id.to_owned()],
                    )])),
                }))
                .await
                .map_err(Error::Docker)?;

            if tasks.is_empty() {
                // A task hasn't been created for the service yet, query again after a delay
                sleep(Duration::from_millis(100)).await;
                continue;
            }

            assert_eq!(
                tasks.len(),
                1,
                "Docker service task count should always be 1"
            );

            let task = tasks.into_iter().next().unwrap();
            let status = task
                .status
                .expect("Docker daemon reported a task with no status");

            match status.state {
                Some(TaskState::NEW)
                | Some(TaskState::PENDING)
                | Some(TaskState::ALLOCATED)
                | Some(TaskState::ASSIGNED)
                | Some(TaskState::ACCEPTED)
                | Some(TaskState::READY)
                | Some(TaskState::PREPARING)
                | None => {
                    trace!(
                        "task has not yet started for service `{id}` (task `{name}`)",
                        id = self.id
                    );

                    // Query again after a delay
                    // TODO: make this a variable delay so as to lessen a thundering herd
                    sleep(Duration::from_secs(1)).await;
                }
                Some(TaskState::STARTING) | Some(TaskState::RUNNING) => {
                    // Wait for the container to exit
                    let status = status.container_status.expect(
                        "Docker daemon reported a starting or running task with no container \
                         status",
                    );

                    let container_id = status
                        .container_id
                        .expect("Docker reported a starting or running task with no container id");

                    info!(
                        "service `{id}` (task `{name}`) has started container `{container_id}",
                        id = self.id
                    );

                    // Notify that the task has started
                    started();

                    // Wait for the container to be completed.
                    let mut wait_stream = self
                        .client
                        .wait_container(&container_id, None::<WaitContainerOptions>);

                    let mut exit_code = None;
                    if let Some(result) = wait_stream.next().await {
                        match result {
                            // Bollard turns non-zero exit codes into wait errors, so check for both
                            Ok(ContainerWaitResponse {
                                status_code: code, ..
                            })
                            | Err(bollard::errors::Error::DockerContainerWaitError {
                                code, ..
                            }) => {
                                exit_code = Some(code);
                            }
                            Err(e) => return Err(e.into()),
                        }
                    }

                    if exit_code.is_none() {
                        // Get the exit code if the wait was immediate
                        let container = self
                            .client
                            .inspect_container(&container_id, None::<InspectContainerOptions>)
                            .await
                            .map_err(Error::Docker)?;

                        exit_code = Some(
                            container
                                .state
                                .expect("Docker reported a container without a state")
                                .exit_code
                                .expect(
                                    "Docker reported a finished contained without an exit code",
                                ),
                        );
                    }

                    break (container_id, exit_code.unwrap());
                }
                Some(TaskState::COMPLETE) => {
                    let status = status
                        .container_status
                        .expect("Docker daemon reported a completed task with no container status");

                    let container_id = status
                        .container_id
                        .expect("Docker reported a completed task with no container id");

                    info!(
                        "container `{container_id}` for service `{id}` (task `{name}`) has \
                         completed",
                        id = self.id
                    );

                    // Notify that the task has started (and has already completed)
                    started();

                    break (
                        container_id,
                        // Use the exit code already provided to us
                        status
                            .exit_code
                            .expect("Docker reported a completed task with no exit code"),
                    );
                }
                Some(TaskState::FAILED)
                | Some(TaskState::SHUTDOWN)
                | Some(TaskState::REJECTED)
                | Some(TaskState::ORPHANED)
                | Some(TaskState::REMOVE) => {
                    // Notify that the task has started (and has failed)
                    started();

                    // Handle the failure
                    return Err(Error::Message(format!(
                        "Docker task failed: {msg}",
                        msg = status
                            .err
                            .as_deref()
                            .or(status.message.as_deref())
                            .unwrap_or("no error message was provided by the Docker daemon")
                    )));
                }
            }
        };

        // Write the log streams
        if self.stdout.is_some() || self.stderr.is_some() {
            let mut stdout = match &self.stdout {
                Some(path) => Some(File::create(path).await.map_err(|e| {
                    Error::Message(format!(
                        "failed to create stdout file `{path}`: {e}",
                        path = path.display()
                    ))
                })?),
                None => None,
            };

            let mut stderr = match &self.stderr {
                Some(path) => Some(File::create(path).await.map_err(|e| {
                    Error::Message(format!(
                        "failed to create stderr file `{path}`: {e}",
                        path = path.display()
                    ))
                })?),
                None => None,
            };

            let mut stream = self.client.logs(
                &container_id,
                Some(LogsOptions {
                    stdout: self.stdout.is_some(),
                    stderr: self.stderr.is_some(),
                    ..Default::default()
                }),
            );

            while let Some(result) = stream.next().await {
                let output = result.map_err(Error::Docker)?;
                match output {
                    LogOutput::StdOut { message } => {
                        stdout
                            .as_mut()
                            .unwrap()
                            .write(&message)
                            .await
                            .map_err(|e| {
                                Error::Message(format!(
                                    "failed to write to stdout file `{path}`: {e}",
                                    path = self.stdout.as_ref().unwrap().display()
                                ))
                            })?;
                    }
                    LogOutput::StdErr { message } => {
                        stderr
                            .as_mut()
                            .unwrap()
                            .write(&message)
                            .await
                            .map_err(|e| {
                                Error::Message(format!(
                                    "failed to write to stderr file `{path}`: {e}",
                                    path = self.stderr.as_ref().unwrap().display()
                                ))
                            })?;
                    }
                    _ => {}
                }
            }
        }

        // See WEXITSTATUS from wait(2) to explain the shift
        #[cfg(unix)]
        let status = ExitStatus::from_raw((exit_code as i32) << 8);

        #[cfg(windows)]
        let status = ExitStatus::from_raw(exit_code as u32);

        Ok(status)
    }

    /// Deletes a service.
    pub async fn delete(&self) -> Result<()> {
        debug!("deleting Docker service `{id}`", id = self.id);
        self.client
            .delete_service(&self.id)
            .await
            .map_err(Error::Docker)?;

        Ok(())
    }
}
