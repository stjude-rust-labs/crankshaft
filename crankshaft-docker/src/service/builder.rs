//! Builders for containers.

use bollard::Docker;
use bollard::secret::Mount;
use bollard::secret::ServiceSpec;
use bollard::secret::ServiceSpecMode;
use bollard::secret::ServiceSpecModeReplicated;
use bollard::secret::TaskSpec;
use bollard::secret::TaskSpecContainerSpec;
use bollard::secret::TaskSpecResources;
use bollard::secret::TaskSpecRestartPolicy;
use bollard::secret::TaskSpecRestartPolicyConditionEnum;
use indexmap::IndexMap;
use tracing::warn;

use super::Service;
use crate::Error;
use crate::Result;

/// A builder for a [`Service`].
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

    /// Whether or not the standard output is attached.
    attach_stdout: bool,

    /// Whether or not the standard error is attached.
    attach_stderr: bool,

    /// Environment variables.
    env: IndexMap<String, String>,

    /// The working directory.
    work_dir: Option<String>,

    /// The mounts for the service's task template.
    mounts: Vec<Mount>,

    /// The task resources for the service.
    resources: Option<TaskSpecResources>,
}

impl Builder {
    /// Creates a new [`Builder`].
    pub fn new(client: Docker) -> Self {
        Self {
            client,
            image: Default::default(),
            program: Default::default(),
            args: Default::default(),
            attach_stdout: false,
            attach_stderr: false,
            env: Default::default(),
            work_dir: Default::default(),
            mounts: Default::default(),
            resources: Default::default(),
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

    /// Sets stdout to be attached.
    pub fn attach_stdout(mut self) -> Self {
        self.attach_stdout = true;
        self
    }

    /// Sets stderr to be attached.
    pub fn attach_stderr(mut self) -> Self {
        self.attach_stderr = true;
        self
    }

    /// Sets the working directory.
    pub fn work_dir(mut self, work_dir: impl Into<String>) -> Self {
        self.work_dir = Some(work_dir.into());
        self
    }

    /// Sets a mount for the service.
    pub fn mount(mut self, mount: impl Into<Mount>) -> Self {
        self.mounts.push(mount.into());
        self
    }

    /// Sets multiple mounts for the service.
    pub fn mounts(mut self, mounts: impl IntoIterator<Item = impl Into<Mount>>) -> Self {
        self.mounts.extend(mounts.into_iter().map(Into::into));
        self
    }

    /// Sets the task resources.
    pub fn resources(mut self, resources: TaskSpecResources) -> Self {
        self.resources = Some(resources);
        self
    }

    /// Consumes `self` and attempts to create a Docker service.
    pub async fn try_build(self, name: impl Into<String>) -> Result<Service> {
        let image = self
            .image
            .ok_or_else(|| Error::MissingBuilderField("image"))?;
        let program = self
            .program
            .ok_or_else(|| Error::MissingBuilderField("program"))?;

        let response = self
            .client
            .create_service(
                ServiceSpec {
                    name: Some(name.into()),
                    mode: Some(ServiceSpecMode {
                        replicated: Some(ServiceSpecModeReplicated { replicas: Some(1) }),
                        ..Default::default()
                    }),
                    task_template: Some(TaskSpec {
                        container_spec: Some(TaskSpecContainerSpec {
                            image: Some(image),
                            command: Some(vec![program]),
                            args: Some(self.args),
                            dir: self.work_dir,
                            env: Some(self.env.iter().map(|(k, v)| format!("{k}={v}")).collect()),
                            mounts: Some(self.mounts),
                            ..Default::default()
                        }),
                        resources: self.resources,
                        restart_policy: Some(TaskSpecRestartPolicy {
                            condition: Some(TaskSpecRestartPolicyConditionEnum::NONE),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                None,
            )
            .await
            .map_err(Error::Docker)?;

        for warning in response.warnings.unwrap_or_default() {
            warn!("Docker daemon: {warning}");
        }

        Ok(Service {
            client: self.client,
            id: response.id.expect("service must have an identifier"),
            attach_stdout: self.attach_stdout,
            attach_stderr: self.attach_stderr,
        })
    }
}
