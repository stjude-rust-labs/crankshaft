//! A unit of executable work.

use std::collections::HashMap;

use bon::Builder;
use indexmap::IndexMap;

/// An execution.
#[derive(Builder, Clone, Debug)]
#[builder(builder_type = Builder)]
pub struct Execution {
    /// The container image.
    #[builder(into)]
    pub(crate) image: String,

    /// The program to execute.
    #[builder(into)]
    pub(crate) program: String,

    /// The arguments to the program.
    #[builder(into, default)]
    pub(crate) args: Vec<String>,

    /// The working directory, if configured.
    #[builder(into)]
    pub(crate) work_dir: Option<String>,

    /// The path inside the container to a file whose contents will be piped to
    /// the standard input, if configured.
    #[builder(into)]
    pub(crate) stdin: Option<String>,

    /// The path inside the container to a file where the contents of the
    /// standard output stream will be written, if configured.
    #[builder(into)]
    pub(crate) stdout: Option<String>,

    /// The path inside the container to a file where the contents of the
    /// standard error stream will be written, if configured.
    #[builder(into)]
    pub(crate) stderr: Option<String>,

    /// A map of environment variables, if configured.
    #[builder(into, default)]
    pub(crate) env: IndexMap<String, String>,
}

impl Execution {
    /// The image for the execution to run within.
    pub fn image(&self) -> &str {
        &self.image
    }

    /// The program to execute.
    pub fn program(&self) -> &str {
        &self.program
    }

    /// The arguments to the execution.
    pub fn args(&self) -> &[String] {
        &self.args
    }

    /// The working directory.
    pub fn work_dir(&self) -> Option<&str> {
        self.work_dir.as_deref()
    }

    /// The file to pipe the standard input stream from.
    pub fn stdin(&self) -> Option<&str> {
        self.stdin.as_deref()
    }

    /// The file to pipe the standard output stream to.
    pub fn stdout(&self) -> Option<&str> {
        self.stdout.as_deref()
    }

    /// The file to pipe the standard error stream to.
    pub fn stderr(&self) -> Option<&str> {
        self.stderr.as_deref()
    }

    /// The environment variables for the execution.
    pub fn env(&self) -> &IndexMap<String, String> {
        &self.env
    }
}

impl From<Execution> for tes::v1::types::task::Executor {
    fn from(execution: Execution) -> Self {
        let env = execution
            .env
            .into_iter()
            .collect::<HashMap<String, String>>();

        let env = if env.is_empty() { None } else { Some(env) };

        let mut command = Vec::with_capacity(execution.args.len() + 1);
        command.push(execution.program);
        command.extend(execution.args);

        tes::v1::types::task::Executor {
            image: execution.image.to_owned(),
            command,
            workdir: execution.work_dir,
            stdin: execution.stdin,
            stdout: execution.stdout,
            stderr: execution.stderr,
            env,
        }
    }
}
