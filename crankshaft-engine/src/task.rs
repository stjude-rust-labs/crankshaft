//! Tasks that can be run by execution runners.

use std::sync::Arc;

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
    name: Option<String>,

    /// An optional description.
    #[builder(into)]
    description: Option<String>,

    /// An optional list of [`Input`]s.
    #[builder(into, default)]
    inputs: Vec<Arc<Input>>,

    /// An optional list of [`Output`]s.
    #[builder(into, default)]
    outputs: Vec<Output>,

    /// An optional set of requested [`Resources`].
    #[builder(into)]
    resources: Option<Resources>,

    /// The list of [`Execution`]s.
    #[builder(into)]
    executions: NonEmpty<Execution>,

    /// The list of volumes shared across executions in the task.
    #[builder(into, default)]
    volumes: Vec<String>,
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
    pub fn inputs(&self) -> impl Iterator<Item = Arc<Input>> + use<'_> {
        self.inputs.iter().cloned()
    }

    /// Gets the outputs for the task (if any exist).
    pub fn outputs(&self) -> impl Iterator<Item = &Output> {
        self.outputs.iter()
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
    pub fn shared_volumes(&self) -> impl Iterator<Item = &str> {
        self.volumes.iter().map(|v| v.as_str())
    }
}

impl From<Task> for tes::v1::types::Task {
    fn from(task: Task) -> Self {
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
            .map(|input| TesInput::from((*input).clone()))
            .collect::<Vec<TesInput>>();

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

        tes::v1::types::Task {
            name,
            description,
            inputs,
            outputs,
            executors,
            resources,
            ..Default::default()
        }
    }
}
