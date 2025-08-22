//! Builders for containers.

use std::path::PathBuf;

use bollard::Docker;
use bollard::models::ContainerCreateBody;
use bollard::query_parameters::CreateContainerOptions;
use bollard::secret::HostConfig;
use indexmap::IndexMap;
use tracing::warn;

use crate::Container;
use crate::Error;
use crate::Result;

/// A builder for a [`Container`].
pub struct Builder {
    /// A reference to the [`Docker`] client that will be used to create this
    /// container.
    client: Docker,

    /// The image (e.g., `ubuntu:latest`).
    image: Option<String>,

    /// The program to run.
    program: Option<String>,

    /// The arguments to the command.
    args: Vec<String>,

    /// The file path to write the container's stdout stream to.
    stdout: Option<PathBuf>,

    /// The file path to write the container's stderr stream to.
    stderr: Option<PathBuf>,

    /// Environment variables.
    env: IndexMap<String, String>,

    /// The working directory.
    work_dir: Option<String>,

    /// Host configuration.
    host_config: Option<HostConfig>,
}

impl Builder {
    /// Creates a new [`Builder`].
    pub fn new(client: Docker) -> Self {
        Self {
            client,
            image: Default::default(),
            program: Default::default(),
            args: Default::default(),
            stdout: None,
            stderr: None,
            env: Default::default(),
            work_dir: Default::default(),
            host_config: Default::default(),
        }
    }

    /// Adds an image name.
    pub fn image(mut self, image: impl Into<String>) -> Self {
        self.image = Some(image.into());
        self
    }

    /// Sets the program to run.
    pub fn program(mut self, program: impl Into<String>) -> Self {
        self.program = Some(program.into());
        self
    }

    /// Sets an argument.
    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Sets multiple arguments.
    pub fn args(mut self, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    /// Sets the file to write the container's stdout stream to.
    pub fn stdout(mut self, path: impl Into<PathBuf>) -> Self {
        self.stdout = Some(path.into());
        self
    }

    /// Sets the file to write the container's stderr stream to.
    pub fn stderr(mut self, path: impl Into<PathBuf>) -> Self {
        self.stderr = Some(path.into());
        self
    }

    /// Sets an environment variable.
    pub fn env(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(name.into(), value.into());
        self
    }

    /// Sets multiple environment variables.
    pub fn envs(
        mut self,
        variables: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        self.env
            .extend(variables.into_iter().map(|(k, v)| (k.into(), v.into())));
        self
    }

    /// Sets the working directory.
    pub fn work_dir(mut self, work_dir: impl Into<String>) -> Self {
        self.work_dir = Some(work_dir.into());
        self
    }

    /// Sets the host configuration.
    pub fn host_config(mut self, host_config: HostConfig) -> Self {
        self.host_config = Some(host_config);
        self
    }

    /// Consumes `self` and attempts to create a Docker container.
    ///
    /// Note that the creation of a container does not start the container.
    pub async fn try_build(self, name: impl Into<String>) -> Result<Container> {
        let image = self
            .image
            .ok_or_else(|| Error::MissingBuilderField("image"))?;
        let program = self
            .program
            .ok_or_else(|| Error::MissingBuilderField("program"))?;

        let mut cmd = Vec::with_capacity(1 + self.args.len());
        cmd.push(program);
        cmd.extend(self.args);

        let name = name.into();
        let response = self
            .client
            .create_container(
                Some(CreateContainerOptions {
                    name: Some(name.clone()),
                    ..Default::default()
                }),
                ContainerCreateBody {
                    // NOTE: even though the following fields are optional, I
                    // want _this_ struct to require the explicit designation
                    // one way or the other and not rely on the default.
                    cmd: Some(cmd),
                    image: Some(image),
                    // Override the entrypoint to the default Docker entrypoint as we're providing
                    // the full command
                    entrypoint: Some(vec![String::new()]),
                    attach_stdout: Some(self.stdout.is_some()),
                    attach_stderr: Some(self.stderr.is_some()),
                    // END NOTE
                    working_dir: self.work_dir,
                    host_config: self.host_config,
                    env: Some(self.env.iter().map(|(k, v)| format!("{k}={v}")).collect()),
                    ..Default::default()
                },
            )
            .await
            .map_err(Error::Docker)?;

        for warning in &response.warnings {
            warn!("{warning}");
        }

        Ok(Container::new(
            self.client,
            response.id,
            self.stdout,
            self.stderr,
        ))
    }
}
