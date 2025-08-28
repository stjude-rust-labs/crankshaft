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
use bollard::query_parameters::InspectContainerOptions;
use bollard::query_parameters::ListTasksOptions;
use bollard::query_parameters::LogsOptionsBuilder;
use bollard::query_parameters::WaitContainerOptions;
use bollard::secret::ContainerWaitResponse;
use bollard::secret::TaskState;

mod builder;

pub use builder::Builder;
use crankshaft_events::Event;
use futures::StreamExt;
use tokio::fs::File;
use tokio::time::sleep;
use tracing::debug;
use tracing::info;
use tracing::trace;

use crate::Error;
use crate::EventOptions;
use crate::Result;
use crate::container::write_logs;

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

    /// The id of the service.
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

    /// Gets the id of the service.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Runs a service and waits for the task execution to end.
    pub async fn run(&self, task_name: &str, events: Option<EventOptions>) -> Result<ExitStatus> {
        let stdout = match &self.stdout {
            Some(path) => Some((
                path.as_path(),
                File::create(path).await.map_err(|e| {
                    Error::Message(format!(
                        "failed to create stdout file `{path}`: {e}",
                        path = path.display()
                    ))
                })?,
            )),
            None => None,
        };

        let stderr = match &self.stderr {
            Some(path) => Some((
                path.as_path(),
                File::create(path).await.map_err(|e| {
                    Error::Message(format!(
                        "failed to create stderr file `{path}`: {e}",
                        path = path.display()
                    ))
                })?,
            )),
            None => None,
        };

        let (container_id, exit_code) = loop {
            trace!(
                "polling tasks for service `{id}` (task `{task_name}`)",
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

            let status = task.status.ok_or_else(|| {
                Error::Message("Docker daemon reported a task with no status".into())
            })?;

            match status.state {
                Some(TaskState::NEW)
                | Some(TaskState::PENDING)
                | Some(TaskState::ALLOCATED)
                | Some(TaskState::ASSIGNED)
                | Some(TaskState::ACCEPTED)
                | Some(TaskState::READY)
                | Some(TaskState::PREPARING)
                | Some(TaskState::STARTING)
                | None => {
                    trace!(
                        "task has not yet started for service `{id}` (task `{task_name}`)",
                        id = self.id
                    );

                    // Query again after a delay
                    // TODO: make this a variable delay so as to lessen a thundering herd
                    sleep(Duration::from_secs(1)).await;
                }
                Some(TaskState::RUNNING) | Some(TaskState::COMPLETE) | Some(TaskState::FAILED) => {
                    let container_status = status.container_status.ok_or_else(|| {
                        Error::Message(
                            "Docker daemon reported a task with no container status".into(),
                        )
                    })?;

                    let container_id = container_status.container_id.ok_or_else(|| {
                        Error::Message("Docker reported a task with no container id".into())
                    })?;

                    if let Some(events) = &events {
                        events
                            .sender
                            .send(Event::TaskContainerCreated {
                                id: events.task_id,
                                container: container_id.clone(),
                            })
                            .ok();
                    }

                    info!(
                        "service `{id}` (task `{task_name}`) has started container `{container_id}",
                        id = self.id
                    );

                    if let Some(events) = &events {
                        if events.send_start {
                            events
                                .sender
                                .send(Event::TaskStarted { id: events.task_id })
                                .ok();
                        }
                    }

                    // Write the logs
                    if self.stdout.is_some() || self.stderr.is_some() {
                        let logs = self.client.logs(
                            &container_id,
                            Some(
                                LogsOptionsBuilder::new()
                                    .stdout(true)
                                    .stderr(true)
                                    .follow(true)
                                    .build(),
                            ),
                        );

                        // Write the logs
                        write_logs(logs, stdout, stderr, events.as_ref()).await?;
                    }

                    if status.state == Some(TaskState::RUNNING) {
                        // Wait for the container to be completed.
                        let mut wait_stream = self
                            .client
                            .wait_container(&container_id, None::<WaitContainerOptions>);

                        match wait_stream.next().await {
                            Some(Ok(ContainerWaitResponse {
                                status_code: code, ..
                            }))
                            | Some(Err(bollard::errors::Error::DockerContainerWaitError {
                                code,
                                ..
                            })) => {
                                break (container_id, code);
                            }
                            Some(Err(e)) => return Err(e.into()),
                            None => {
                                // Get the exit code if the wait was immediate
                                let container = self
                                    .client
                                    .inspect_container(
                                        &container_id,
                                        None::<InspectContainerOptions>,
                                    )
                                    .await
                                    .map_err(Error::Docker)?;

                                break (
                                    container_id,
                                    container
                                        .state
                                        .ok_or_else(|| {
                                            Error::Message(
                                                "Docker reported a container without a state"
                                                    .into(),
                                            )
                                        })?
                                        .exit_code
                                        .ok_or_else(|| {
                                            Error::Message(
                                                "Docker reported a finished contained without an \
                                                 exit code"
                                                    .into(),
                                            )
                                        })?,
                                );
                            }
                        }
                    } else {
                        break (
                            container_id,
                            container_status.exit_code.ok_or_else(|| {
                                Error::Message(format!(
                                    "Docker reported a {kind} task with no exit code",
                                    kind = if status.state == Some(TaskState::FAILED) {
                                        "failed"
                                    } else {
                                        "completed"
                                    }
                                ))
                            })?,
                        );
                    }
                }
                Some(TaskState::SHUTDOWN)
                | Some(TaskState::REJECTED)
                | Some(TaskState::ORPHANED)
                | Some(TaskState::REMOVE) => {
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

        // See WEXITSTATUS from wait(2) to explain the shift
        #[cfg(unix)]
        let status = ExitStatus::from_raw((exit_code as i32) << 8);

        #[cfg(windows)]
        let status = ExitStatus::from_raw(exit_code as u32);

        info!(
            "container `{container_id}` for service `{id}` (task `{task_name}`) has exited with \
             {status}",
            id = self.id
        );

        if let Some(events) = &events {
            events
                .sender
                .send(Event::TaskContainerExited {
                    id: events.task_id,
                    container: container_id,
                    exit_status: status,
                })
                .ok();
        }

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
