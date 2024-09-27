//! Tasks that can be run by execution runners.

use nonempty::NonEmpty;

mod builder;
pub mod execution;
pub mod input;
pub mod output;
pub mod resources;

pub use builder::Builder;
pub use execution::Execution;
pub use input::Input;
pub use output::Output;
pub use resources::Resources;

/// A task intended for execution.
#[derive(Clone, Debug)]

pub struct Task {
    /// An optional name.
    name: Option<String>,

    /// An optional description.
    description: Option<String>,

    /// An optional list of [`Input`]s.
    inputs: Option<NonEmpty<Input>>,

    /// An optional list of [`Output`]s.
    outputs: Option<NonEmpty<Output>>,

    /// An optional set of requested [`Resources`].
    resources: Option<Resources>,

    /// The list of [`Execution`]s.
    executions: NonEmpty<Execution>,

    /// The list of volumes shared across executions in the task.
    shared_volumes: Option<NonEmpty<String>>,
}

impl Task {
    /// Gets a new task builder.
    pub fn builder() -> Builder {
        Builder::default()
    }

    /// Gets the name of the task (if it exists).
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Overrides a task's name (regardless of if it previously existed or not).
    pub fn override_name(&mut self, name: String) {
        self.name = Some(name)
    }

    /// Gets the description of the task (if it exists).
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Gets the inputs for the task (if any exist).
    pub fn inputs(&self) -> Option<impl Iterator<Item = &Input>> {
        self.inputs.as_ref().map(|inputs| inputs.iter())
    }

    /// Gets the outputs for the task (if any exist).
    pub fn outputs(&self) -> Option<impl Iterator<Item = &Output>> {
        self.outputs.as_ref().map(|outputs| outputs.iter())
    }

    /// Gets the requested resources for the task (if any are specified).
    pub fn resources(&self) -> Option<&Resources> {
        self.resources.as_ref()
    }

    /// Gets the executions for this task.
    pub fn executions(&self) -> impl Iterator<Item = &Execution> {
        self.executions.iter()
    }

    /// Gets the shared volumes across executions within this task.
    pub fn shared_volumes(&self) -> Option<impl Iterator<Item = &str>> {
        self.shared_volumes
            .as_ref()
            .map(|volumes| volumes.iter().map(|a| a.as_str()))
    }
}
