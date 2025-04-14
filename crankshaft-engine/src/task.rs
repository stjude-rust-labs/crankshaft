//! Tasks that can be run by execution runners.

use bon::Builder;
use nonempty::NonEmpty;
use tes::v1::types::task::Executor;
use tes::v1::types::task::Input as TesInput;
use tes::v1::types::task::Output as TesOutput;
use tes::v1::types::task::Resources as TesResources;

pub mod execution;
pub mod input;
pub mod output;
pub mod resources;

pub use execution::Execution;
pub use input::Input;
pub use output::Output;
pub use resources::Resources;

/// A task intended for execution.
#[derive(Builder, Clone, Debug)]
#[builder(builder_type = Builder)]
pub struct Task {
    /// An optional name.
    #[builder(into)]
    pub(crate) name: Option<String>,

    /// An optional description.
    #[builder(into)]
    pub(crate) description: Option<String>,

    /// An optional list of [`Input`]s.
    #[builder(into, default)]
    pub(crate) inputs: Vec<Input>,

    /// An optional list of [`Output`]s.
    #[builder(into, default)]
    pub(crate) outputs: Vec<Output>,

    /// An optional set of requested [`Resources`].
    #[builder(into)]
    pub(crate) resources: Option<Resources>,

    /// The list of [`Execution`]s.
    #[builder(into)]
    pub(crate) executions: NonEmpty<Execution>,

    /// The list of volumes shared across executions in the task.
    #[builder(into, default)]
    pub(crate) volumes: Vec<String>,
}

impl Task {
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
    pub fn inputs(&self) -> impl Iterator<Item = &Input> {
        self.inputs.iter()
    }

    /// Adds an input to the task.
    pub fn add_input(&mut self, input: impl Into<Input>) {
        self.inputs.push(input.into());
    }

    /// Gets the outputs for the task (if any exist).
    pub fn outputs(&self) -> impl Iterator<Item = &Output> {
        self.outputs.iter()
    }

    /// Adds an output to the task.
    pub fn add_output(&mut self, output: impl Into<Output>) {
        self.outputs.push(output.into());
    }

    /// Gets the requested resources for the task (if any are specified).
    pub fn resources(&self) -> Option<&Resources> {
        self.resources.as_ref()
    }

    /// Gets the executions for this task.
    pub fn executions(&self) -> impl Iterator<Item = &Execution> {
        self.executions.iter()
    }

    /// Adds an execution to the task.
    pub fn add_execution(&mut self, execution: impl Into<Execution>) {
        self.executions.push(execution.into());
    }

    /// Gets the shared volumes across executions within this task.
    pub fn shared_volumes(&self) -> impl Iterator<Item = &str> {
        self.volumes.iter().map(|v| v.as_str())
    }
}

impl TryFrom<Task> for tes::v1::types::requests::Task {
    type Error = eyre::Error;

    fn try_from(task: Task) -> Result<Self, Self::Error> {
        let Task {
            name,
            description,
            inputs,
            outputs,
            resources,
            executions,
            volumes,
        } = task;

        //========//
        // Inputs //
        //========//

        let inputs = inputs
            .into_iter()
            .map(TesInput::try_from)
            .collect::<eyre::Result<Vec<_>>>()?;

        let inputs = if inputs.is_empty() {
            None
        } else {
            Some(inputs)
        };

        //=========//
        // Outputs //
        //=========//

        let outputs = outputs
            .into_iter()
            .map(|output| TesOutput::from(output.clone()))
            .collect::<Vec<TesOutput>>();

        let outputs = if outputs.is_empty() {
            None
        } else {
            Some(outputs)
        };

        //============//
        // Executions //
        //============//

        let executors = executions.map(Executor::from).into_iter().collect();

        //===========//
        // Resources //
        //===========//

        let resources = resources.map(TesResources::from);

        //=========//
        // Volumes //
        //=========//

        if !volumes.is_empty() {
            todo!("volumes are not yet supported within Crankshaft");
        }

        Ok(Self {
            name,
            description,
            inputs,
            outputs,
            executors,
            resources,
            ..Default::default()
        })
    }
}
