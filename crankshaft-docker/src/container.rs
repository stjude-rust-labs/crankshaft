//! Containers.

mod builder;

use std::io::Cursor;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt as _;
#[cfg(windows)]
use std::os::windows::process::ExitStatusExt as _;
use std::process::ExitStatus;
use std::process::Output;

use bollard::Docker;
use bollard::container::AttachContainerOptions;
use bollard::container::LogOutput;
use bollard::container::RemoveContainerOptions;
use bollard::container::StartContainerOptions;
use bollard::container::UploadToContainerOptions;
use bollard::container::WaitContainerOptions;
pub use builder::Builder;
use futures::TryStreamExt as _;
use tokio_stream::StreamExt as _;
use tracing::Level;
use tracing::debug;
use tracing::enabled;
use tracing::trace;

use crate::Error;
use crate::Result;

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

    /// The name of the container.
    name: String,

    /// Whether or not standard output and standard error were attached.
    attached: bool,
}

impl Container {
    /// Gets a new creator for a [`Container`].
    pub fn builder(client: Docker) -> Builder {
        Builder::new(client)
    }

    /// Creates a new [`Container`] if you already know the name of a container.
    ///
    /// You should typically use [`Self::builder()`] unless you receive the
    /// container name externally from a user (say, on the command line as an
    /// argument).
    pub fn new(client: Docker, name: String, attached: bool) -> Self {
        Self {
            client,
            name,
            attached,
        }
    }

    /// Uploads an input file to the container.
    pub async fn upload_file(&self, path: &str, contents: Vec<u8>) -> Result<()> {
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
                    path: "/",
                    ..Default::default()
                }),
                // SAFETY: this is manually crafted to always unwrap.
                tar.into_inner().unwrap().into(),
            )
            .await
            .map_err(Error::Docker)
    }

    /// Runs a container and waits for the execution to end.
    pub async fn run(&self) -> Result<Output> {
        // (1) Attach to the logs stream.
        let stream = self
            .client
            .attach_container(
                &self.name,
                Some(AttachContainerOptions::<String> {
                    stdout: Some(self.attached),
                    stderr: Some(self.attached),
                    stream: Some(true),
                    ..Default::default()
                }),
            )
            .await
            .map_err(Error::Docker)?
            .output;

        // (2) Start the container.
        self.client
            .start_container(&self.name, None::<StartContainerOptions<String>>)
            .await
            .map_err(Error::Docker)?;

        // (3) Collect standard out/standard err.
        let (stdout, stderr) = stream
            .try_fold(
                (
                    Vec::<u8>::with_capacity(0x0FFF),
                    Vec::<u8>::with_capacity(0x0FFF),
                ),
                |(mut stdout, mut stderr), log| async move {
                    match log {
                        LogOutput::StdOut { message } => {
                            stdout.extend(&message);
                        }
                        LogOutput::StdErr { message } => {
                            stderr.extend(&message);
                        }
                        v => {
                            trace!("unhandled log message: {v:?}")
                        }
                    }

                    Ok((stdout, stderr))
                },
            )
            .await
            .map_err(Error::Docker)?;

        // (4) Wait for the container to be completed.
        let mut wait_stream = self
            .client
            .wait_container(&self.name, None::<WaitContainerOptions<String>>);

        while let Some(result) = wait_stream.next().await {
            let response = result.map_err(Error::Docker)?;

            if enabled!(Level::TRACE) {
                trace!("{response:?}");
            }
        }

        // (5) Get the exit code.
        let inspect = self
            .client
            .inspect_container(&self.name, None)
            .await
            .map_err(Error::Docker)?;

        let status = inspect
            .state
            .expect("state should be present at this point")
            .exit_code
            .expect("exit code should be present at this point") as i32;

        #[cfg(unix)]
        let output = Output {
            status: ExitStatus::from_raw(status),
            stdout,
            stderr,
        };

        #[cfg(windows)]
        let output = Output {
            status: ExitStatus::from_raw(status as u32),
            stdout,
            stderr,
        };

        Ok(output)
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
        debug!("removing container: `{}`", self.name);
        self.remove_inner(false).await
    }

    /// Removes a container with force.
    ///
    /// This forces the container to be removed. To unforcefully remove the
    /// container, see the [`Self::remove()`] method.
    pub async fn force_remove(&self) -> Result<()> {
        debug!("force removing container: `{}`", self.name);
        self.remove_inner(true).await
    }
}
