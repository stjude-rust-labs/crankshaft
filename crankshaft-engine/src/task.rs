//! Tasks that can be run by execution runners.

use bon::Builder;
use nonempty::NonEmpty;

pub mod execution;
pub mod input;
pub mod output;
pub mod resources;

pub use execution::Execution;
pub use input::Input;
pub use output::Output;
pub use resources::Resources;
use tes::v1::types::task::Executor;
use tes::v1::types::task::Input as TesInput;
use tes::v1::types::task::Output as TesOutput;
use tes::v1::types::task::Resources as TesResources;

/// A task intended for execution.
#[derive(Builder, Clone, Debug)]
#[builder(builder_type = Builder)]
pub struct Task {
    /// An optional name.
    #[builder(into)]
    pub name: Option<String>,

    /// An optional description.
    #[builder(into)]
    pub description: Option<String>,

    /// An optional list of [`Input`]s.
    #[builder(into, default)]
    pub inputs: Vec<Input>,

    /// An optional list of [`Output`]s.
    #[builder(into, default)]
    pub outputs: Vec<Output>,

    /// An optional set of requested [`Resources`].
    #[builder(into)]
    pub resources: Option<Resources>,

    /// The list of [`Execution`]s.
    #[builder(into)]
    pub executions: NonEmpty<Execution>,

    /// The list of volumes shared across executions in the task.
    #[builder(into, default)]
    pub volumes: Vec<String>,
}

impl TryFrom<Task> for tes::v1::types::Task {
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

        Ok(tes::v1::types::Task {
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
