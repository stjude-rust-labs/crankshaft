//! Containers.

use std::io::Cursor;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt as _;
#[cfg(windows)]
use std::os::windows::process::ExitStatusExt as _;
use std::path::PathBuf;
use std::process::ExitStatus;

use bollard::Docker;
use bollard::body_full;
use bollard::container::LogOutput;
use bollard::query_parameters::AttachContainerOptions;
use bollard::query_parameters::InspectContainerOptions;
use bollard::query_parameters::RemoveContainerOptions;
use bollard::query_parameters::StartContainerOptions;
use bollard::query_parameters::UploadToContainerOptions;
use bollard::query_parameters::WaitContainerOptions;
use bollard::secret::ContainerWaitResponse;
use crankshaft_monitor::proto::Event;
use crankshaft_monitor::proto::EventType;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::broadcast;
use tokio_stream::StreamExt as _;
use tracing::debug;
use tracing::info;

use crate::Error;
use crate::Result;
use crate::events::send_event;

mod builder;

pub use builder::Builder;

/// The default capacity of bytes for a TAR being built.
///
/// It's unlikely that any file we send will be less than this number of
/// bytes, so this is arbitrarily selected to avoid the first few
/// allocations.
const DEFAULT_TAR_CAPACITY: usize = 0xFFFF;

/// A container.
pub struct Container {
    /// A reference to the [`Docker`] client that will be used to create this
    /// container.
    client: Docker,

    /// The id of the created container.
    id: String,

    /// The path to the file to write the container's stdout stream to.
    stdout: Option<PathBuf>,

    /// The path to the file to write the container's stderr stream to.
    stderr: Option<PathBuf>,
}

impl Container {
    /// Creates a new [`Container`] if you already know the container id.
    ///
    /// You should typically use a [`Builder`] unless you receive the container
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
                &self.id,
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
    pub async fn run(
        &self,
        name: &str,
        event_sender: Option<broadcast::Sender<Event>>,
    ) -> Result<ExitStatus> {
        // Attach to the container before we start it
        let stream = if self.stdout.is_some() || self.stderr.is_some() {
            debug!(
                "attaching to container `{id}` (task `{name}`)",
                id = self.id
            );

            // Attach to the logs stream.
            Some(
                self.client
                    .attach_container(
                        &self.id,
                        Some(AttachContainerOptions {
                            stdout: self.stdout.is_some(),
                            stderr: self.stderr.is_some(),
                            stream: true,
                            ..Default::default()
                        }),
                    )
                    .await
                    .map_err(Error::Docker)?
                    .output,
            )
        } else {
            None
        };

        info!("starting container `{id}` (task `{name}`)", id = self.id);

        // Start the container.
        self.client
            .start_container(&self.id, None::<StartContainerOptions>)
            .await
            .map_err(Error::Docker)?;

        let task_id = &self.id;

        info!("container `{id}` (task `{name}`) has started", id = self.id);

        send_event(&event_sender, task_id, EventType::TaskStarted, "");

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

            let mut stream = stream.expect("should have attached to the container");
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

                        send_event(
                            &event_sender,
                            task_id,
                            EventType::TaskStarted,
                            std::str::from_utf8(&message)
                                .expect("Invalid UTF-8")
                                .to_string(),
                        );
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

                        send_event(
                            &event_sender,
                            task_id,
                            EventType::TaskStarted,
                            std::str::from_utf8(&message)
                                .expect("Invalid UTF-8")
                                .to_string(),
                        );
                    }
                    _ => {}
                }
            }
        }

        // Wait for the container to be completed.
        debug!(
            "waiting for container `{id}` (task `{name}`) to exit",
            id = self.id
        );
        let mut wait_stream = self
            .client
            .wait_container(&self.id, None::<WaitContainerOptions>);

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
                .inspect_container(&self.id, None::<InspectContainerOptions>)
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
            "container `{id}` (task `{name}`) has exited with {status}",
            id = self.id
        );

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
                &self.id,
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
        debug!("removing container `{id}`", id = self.id);
        self.remove_inner(false).await
    }

    /// Removes a container with force.
    ///
    /// This forces the container to be removed. To unforcefully remove the
    /// container, see the [`Self::remove()`] method.
    pub async fn force_remove(&self) -> Result<()> {
        debug!("force removing container `{id}`", id = self.id);
        self.remove_inner(true).await
    }
}
