//! A builder for a [`Task`].

use nonempty::NonEmpty;

use crate::Task;
use crate::task::Execution;
use crate::task::Input;
use crate::task::Output;
use crate::task::Resources;

/// An error related to a [`Builder`].
#[derive(Debug)]
pub enum Error {
    /// A required value was missing for a builder field.
    Missing(&'static str),

    /// Multiple values were provided for a singular builder field.
    Multiple(&'static str),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Missing(field) => {
                write!(f, "missing required value for '{field}' in task builder")
            }
            Error::Multiple(field) => {
                write!(f, "multiple value provided for '{field}' in task builder")
            }
        }
    }
}

impl std::error::Error for Error {}

/// A [`Result`](std::result::Result) with an [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// A builder for an [`Task`].
#[derive(Debug, Default)]
pub struct Builder {
    /// An optional name.
    name: Option<String>,

    /// An optional description.
    description: Option<String>,

    /// An optional list of [`Input`]s.
    inputs: Option<NonEmpty<Input>>,

    /// An optional list of [`Output`]s.
    outputs: Option<NonEmpty<Output>>,

    /// An optional set of [`Resources`].
    resources: Option<Resources>,

    /// The list of [`Executor`]s.
    executors: Option<NonEmpty<Execution>>,

    /// The list of volumes shared across executions in the task.
    shared_volumes: Option<NonEmpty<String>>,
}

impl Builder {
    /// Adds a name to the [`Builder`].
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous name declarations provided to
    /// the builder.
    pub fn name<S: Into<String>>(mut self, name: S) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Adds a description to the [`Builder`].
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous description declarations
    /// provided to the builder.
    pub fn description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Extends the set of inputs within the [`Builder`].
    pub fn extend_inputs<Iter>(mut self, inputs: Iter) -> Self
    where
        Iter: IntoIterator<Item = Input>,
    {
        let mut new = inputs.into_iter();

        self.inputs = match self.inputs {
            Some(mut inputs) => {
                inputs.extend(new);
                Some(inputs)
            }
            None => {
                if let Some(input) = new.next() {
                    let mut inputs = NonEmpty::new(input);
                    inputs.extend(new);
                    Some(inputs)
                } else {
                    None
                }
            }
        };

        self
    }

    /// Extends the set of outputs within the [`Builder`].
    pub fn extend_outputs<Iter>(mut self, outputs: Iter) -> Self
    where
        Iter: IntoIterator<Item = Output>,
    {
        let mut new = outputs.into_iter();

        self.outputs = match self.outputs {
            Some(mut outputs) => {
                outputs.extend(new);
                Some(outputs)
            }
            None => {
                if let Some(output) = new.next() {
                    let mut outputs = NonEmpty::new(output);
                    outputs.extend(new);
                    Some(outputs)
                } else {
                    None
                }
            }
        };

        self
    }

    /// Adds a set of requested resources to the [`Builder`].
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous description declarations
    /// provided to the builder.
    pub fn resources<R: Into<Resources>>(mut self, resources: R) -> Self {
        self.resources = Some(resources.into());
        self
    }

    /// Extends the set of executions within the [`Builder`].
    pub fn extend_executions<Iter>(mut self, executors: Iter) -> Self
    where
        Iter: IntoIterator<Item = Execution>,
    {
        let mut new = executors.into_iter();

        self.executors = match self.executors {
            Some(mut executors) => {
                executors.extend(new);
                Some(executors)
            }
            None => {
                if let Some(executor) = new.next() {
                    let mut executors = NonEmpty::new(executor);
                    executors.extend(new);
                    Some(executors)
                } else {
                    None
                }
            }
        };

        self
    }

    /// Extends the set of shared volumes within the [`Builder`].
    pub fn extend_volumes<Iter>(mut self, volumes: Iter) -> Self
    where
        Iter: IntoIterator<Item = String>,
    {
        let mut new = volumes.into_iter();

        self.shared_volumes = match self.shared_volumes {
            Some(mut volumes) => {
                volumes.extend(new);
                Some(volumes)
            }
            None => {
                if let Some(volume) = new.next() {
                    let mut volumes: NonEmpty<_> = NonEmpty::new(volume);
                    volumes.extend(new);
                    Some(volumes)
                } else {
                    None
                }
            }
        };

        self
    }

    /// Consumes `self` and attempts to return a built [`Task`].
    pub fn try_build(self) -> Result<Task> {
        let executors = self
            .executors
            .map(Ok)
            .unwrap_or(Err(Error::Missing("executors")))?;

        Ok(Task {
            name: self.name,
            description: self.description,
            inputs: self.inputs,
            outputs: self.outputs,
            resources: self.resources,
            executions: executors,
            shared_volumes: self.shared_volumes,
        })
    }
}
