//! Builders for containers.

use bollard::Docker;
use bollard::container::Config;
use bollard::container::CreateContainerOptions;
use bollard::secret::HostConfig;
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

    /// The command to run.
    command: Option<Vec<String>>,

    /// Whether or not the output streams (standard output and standard error)
    /// are attached.
    attached: Option<bool>,

    /// Environment variables.
    env: Option<Vec<String>>,

    /// The working directory.
    workdir: Option<String>,

    /// Host configuration.
    host_config: Option<HostConfig>,
}

impl Builder {
    /// Creates a new [`Builder`].
    pub fn new(client: Docker) -> Self {
        Self {
            client,
            image: Default::default(),
            command: Default::default(),
            attached: Default::default(),
            env: Default::default(),
            workdir: Default::default(),
            host_config: Default::default(),
        }
    }

    /// Adds an image name.
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous image name(s) provided to the
    /// builder.
    pub fn image(mut self, image: impl Into<String>) -> Self {
        self.image = Some(image.into());
        self
    }

    /// Sets the command.
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous command(s) provided to the
    /// builder.
    pub fn command(mut self, command: impl Into<Vec<String>>) -> Self {
        self.command = Some(command.into());
        self
    }

    /// Sets whether or not the standard output and standard error will be
    /// attached.
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous attached values provided to
    /// the builder.
    pub fn attached(mut self, attached: bool) -> Self {
        self.attached = Some(attached);
        self
    }

    /// Adds a set of environment variables.
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous command(s) provided to the
    /// builder.
    pub fn extend_env(
        mut self,
        variables: impl Iterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        let mut env = self.env.unwrap_or_default();
        env.extend(
            variables
                .into_iter()
                .map(|(key, value)| format!("{}={}", key.into(), value.into())),
        );
        self.env = Some(env);
        self
    }

    /// Sets the working directory.
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous working directory values
    /// provided to the builder.
    pub fn workdir(mut self, workdir: String) -> Self {
        self.workdir = Some(workdir);
        self
    }

    /// Sets the host configuration.
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous host configuration values
    /// provided to the builder.
    pub fn host_config(mut self, host_config: HostConfig) -> Self {
        self.host_config = Some(host_config);
        self
    }

    /// Consumes `self` and attempts to create a Docker container.
    ///
    /// Note that the creation of a container does not indicate that it has
    /// started.
    pub async fn try_create(self, name: impl AsRef<str>) -> Result<Container> {
        let name = name.as_ref();

        let image = self
            .image
            .expect("the `image` field must be set for a container builder");
        let command = self
            .command
            .expect("the `command` field must be set for a container builder");
        let attached = self
            .attached
            .expect("the `attached` field must be set for a container builder");

        let response = self
            .client
            .create_container(
                Some(CreateContainerOptions {
                    name,
                    ..Default::default()
                }),
                Config {
                    // NOTE: even though the following fields are optional, I
                    // want _this_ struct to require the explicit designation
                    // one way or the other and not rely on the default.
                    cmd: Some(command),
                    image: Some(image),
                    attach_stdout: Some(attached),
                    attach_stderr: Some(attached),
                    // END NOTE
                    working_dir: self.workdir,
                    host_config: self.host_config,
                    env: self.env,
                    ..Default::default()
                },
            )
            .await
            .map_err(Error::Docker)?;

        for warning in &response.warnings {
            warn!("{warning}");
        }

        Ok(Container {
            client: self.client,
            name: response.id,
            attached,
        })
    }
}
