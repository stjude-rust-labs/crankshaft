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
    pub image: String,

    /// The program to execute.
    #[builder(into)]
    pub program: String,

    /// The arguments to the program.
    #[builder(into, default)]
    pub args: Vec<String>,

    /// The working directory, if configured.
    #[builder(into)]
    pub work_dir: Option<String>,

    /// The path inside the container to a file whose contents will be piped to
    /// the standard input, if configured.
    #[builder(into)]
    pub stdin: Option<String>,

    /// The path inside the container to a file where the contents of the
    /// standard output stream will be written, if configured.
    #[builder(into)]
    pub stdout: Option<String>,

    /// The path inside the container to a file where the contents of the
    /// standard error stream will be written, if configured.
    #[builder(into)]
    pub stderr: Option<String>,

    /// The environment variables to set for the execution.
    #[builder(into, default)]
    pub env: IndexMap<String, String>,
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
