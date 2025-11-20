//! Containers.

use std::io::Cursor;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt as _;
#[cfg(windows)]
use std::os::windows::process::ExitStatusExt as _;
use std::path::Path;
use std::path::PathBuf;
use std::process::ExitStatus;

use bollard::Docker;
use bollard::body_full;
use bollard::container::LogOutput;
use bollard::query_parameters::InspectContainerOptions;
use bollard::query_parameters::LogsOptionsBuilder;
use bollard::query_parameters::RemoveContainerOptions;
use bollard::query_parameters::StartContainerOptions;
use bollard::query_parameters::UploadToContainerOptions;
use bollard::query_parameters::WaitContainerOptions;
use bollard::secret::ContainerWaitResponse;
use crankshaft_events::Event;
use futures::Stream;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::pin;
use tokio_stream::StreamExt as _;
use tracing::debug;
use tracing::info;

use crate::Error;
use crate::EventOptions;
use crate::Result;

mod builder;

pub use builder::Builder;

/// The default capacity of bytes for a TAR being built.
///
/// It's unlikely that any file we send will be less than this number of
/// bytes, so this is arbitrarily selected to avoid the first few
/// allocations.
const DEFAULT_TAR_CAPACITY: usize = 0xFFFF;

/// Helper for writing a container's logs to stdout/stderr files.
///
/// This also sends the stdout/stderr events for the task.
pub(crate) async fn write_logs(
    logs: impl Stream<Item = std::result::Result<LogOutput, bollard::errors::Error>>,
    mut stdout: Option<(&Path, File)>,
    mut stderr: Option<(&Path, File)>,
    events: Option<&EventOptions>,
) -> Result<()> {
    pin!(logs);

    while let Some(result) = logs.next().await {
        let output = result.map_err(Error::Docker)?;
        match output {
            LogOutput::StdOut { message } => {
                if let Some((path, stdout)) = &mut stdout {
                    stdout.write(&message).await.map_err(|e| {
                        Error::Message(format!(
                            "failed to write to stdout file `{path}`: {e}",
                            path = path.display()
                        ))
                    })?;
                }

                if let Some(events) = events {
                    events
                        .sender
                        .send(Event::TaskStdout {
                            id: events.task_id,
                            message,
                        })
                        .ok();
                }
            }
            LogOutput::StdErr { message } => {
                if let Some((path, stderr)) = &mut stderr {
                    stderr.write(&message).await.map_err(|e| {
                        Error::Message(format!(
                            "failed to write to stderr file `{path}`: {e}",
                            path = path.display()
                        ))
                    })?;
                }

                if let Some(events) = &events {
                    events
                        .sender
                        .send(Event::TaskStderr {
                            id: events.task_id,
                            message,
                        })
                        .ok();
                }
            }
            _ => {}
        }
    }

    Ok(())
}

/// A container.
pub struct Container {
    /// A reference to the [`Docker`] client that will be used to create this
    /// container.
    client: Docker,

    /// The name of the created container.
    name: String,

    /// The path to the file to write the container's stdout stream to.
    stdout: Option<PathBuf>,

    /// The path to the file to write the container's stderr stream to.
    stderr: Option<PathBuf>,
}

impl Container {
    /// Creates a new [`Container`] if you already know the container name.
    ///
    /// You should typically use a [`Builder`] unless you receive the container
    /// name externally from a user (say, on the command line as an argument).
    pub fn new(
        client: Docker,
        name: String,
        stdout: Option<PathBuf>,
        stderr: Option<PathBuf>,
    ) -> Self {
        Self {
            client,
            name,
            stdout,
            stderr,
        }
    }

    /// Gets the name of the container.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Uploads an input file to the container.
    pub async fn upload_file(&self, path: &str, contents: &[u8]) -> Result<()> {
        let mut tar = tar::Builder::new(Vec::with_capacity(DEFAULT_TAR_CAPACITY));
        let path = path.trim_start_matches("/");

        let mut header = tar::Header::new_gnu();
        header.set_path(path).unwrap();
        header.set_size(contents.len() as u64);
        header.set_mode(0o644);

        // SAFETY: this is manually crafted to always unwrap.
        tar.append_data(&mut header, path, Cursor::new(contents))
            .unwrap();

        self.client
            .upload_to_container(
                &self.name,
                Some(UploadToContainerOptions {
                    path: String::from("/"),
                    ..Default::default()
                }),
                // SAFETY: this is manually crafted to always unwrap.
                body_full(tar.into_inner().unwrap().into()),
            )
            .await
            .map_err(Error::Docker)
    }

    /// Runs a container and waits for the execution to end.
    pub async fn run(&self, task_name: &str, events: Option<EventOptions>) -> Result<ExitStatus> {
        if let Some(events) = &events {
            events
                .sender
                .send(Event::TaskContainerCreated {
                    id: events.task_id,
                    container: self.name.clone(),
                })
                .ok();
        }

        info!(
            "starting container `{name}` (task `{task_name}`)",
            name = self.name
        );

        // Start the container.
        self.client
            .start_container(&self.name, None::<StartContainerOptions>)
            .await
            .map_err(Error::Docker)?;

        info!(
            "container `{name}` (task `{task_name}`) has started",
            name = self.name
        );

        if let Some(events) = &events
            && events.send_start
        {
            events
                .sender
                .send(Event::TaskStarted { id: events.task_id })
                .ok();
        }

        // Write the log streams
        if self.stdout.is_some() || self.stderr.is_some() {
            let logs = self.client.logs(
                &self.name,
                Some(
                    LogsOptionsBuilder::new()
                        .stdout(self.stdout.is_some())
                        .stderr(self.stderr.is_some())
                        .follow(true)
                        .build(),
                ),
            );

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

            write_logs(logs, stdout, stderr, events.as_ref()).await?;
        }

        // Wait for the container to be completed.
        debug!(
            "waiting for container `{name}` (task `{task_name}`) to exit",
            name = self.name
        );
        let mut wait_stream = self
            .client
            .wait_container(&self.name, None::<WaitContainerOptions>);

        let mut exit_code = None;
        if let Some(result) = wait_stream.next().await {
            match result {
                // Bollard turns non-zero exit codes into wait errors, so check for both
                Ok(ContainerWaitResponse {
                    status_code: code, ..
                })
                | Err(bollard::errors::Error::DockerContainerWaitError { code, .. }) => {
                    exit_code = Some(code);
                }
                Err(e) => return Err(e.into()),
            }
        }

        if exit_code.is_none() {
            // Get the exit code if the wait was immediate
            let container = self
                .client
                .inspect_container(&self.name, None::<InspectContainerOptions>)
                .await
                .map_err(Error::Docker)?;

            exit_code = Some(
                container
                    .state
                    .expect("Docker reported a container without a state")
                    .exit_code
                    .expect("Docker reported a finished contained without an exit code"),
            );
        }

        // See WEXITSTATUS from wait(2) to explain the shift
        #[cfg(unix)]
        let status = ExitStatus::from_raw((exit_code.unwrap() as i32) << 8);

        #[cfg(windows)]
        let status = ExitStatus::from_raw(exit_code.unwrap() as u32);

        info!(
            "container `{name}` (task `{task_name}`) has exited with {status}",
            name = self.name
        );

        if let Some(events) = &events {
            events
                .sender
                .send(Event::TaskContainerExited {
                    id: events.task_id,
                    container: self.name.clone(),
                    exit_status: status,
                })
                .ok();
        }

        Ok(status)
    }

    /// Removes a container with the level of force specified.
    ///
    /// This is an inner function, meaning it's not public. There are two public
    /// versions made available: [`Self::remove()`] and
    /// [`Self::force_remove()`].
    async fn remove_inner(&self, force: bool) -> Result<()> {
        self.client
            .remove_container(
                &self.name,
                Some(RemoveContainerOptions {
                    force,
                    ..Default::default()
                }),
            )
            .await
            .map_err(Error::Docker)?;

        Ok(())
    }

    /// Removes a container.
    ///
    /// This does not force the removal of the container. To force the container
    /// to be removed, see the [`Self::force_remove()`] method.
    pub async fn remove(&self) -> Result<()> {
        debug!("removing container `{name}`", name = self.name);
        self.remove_inner(false).await
    }

    /// Removes a container with force.
    ///
    /// This forces the container to be removed. To unforcefully remove the
    /// container, see the [`Self::remove()`] method.
    pub async fn force_remove(&self) -> Result<()> {
        debug!("force removing container `{name}`", name = self.name);
        self.remove_inner(true).await
    }
}
