//! An example for runner a task using the generic LSF backend service.
//!
//! You can run this command with the following command:
//!
//! `cargo run --release --example lsf`

use clap::Parser;
use crankshaft::Config;
use crankshaft::Engine;
use crankshaft::engine::Task;
use crankshaft::engine::task::Execution;
use eyre::Context as _;
use eyre::ContextCompat as _;
use eyre::Result;
use tracing::info;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::util::SubscriberInitExt as _;

#[derive(Debug, Parser)]
#[allow(missing_docs)]
pub struct Args {
    /// The number of jobs to submit in total.
    #[arg(short, long, default_value_t = 1000)]
    n_jobs: usize,
}

/// Simulating a configuration file for LSF using the generic execution backend.
const CONFIG: &str = r#"backends:
  - name: lsf
    kind: Generic
    attributes:
      hosts: '1'
    defaults:
      ram: 3
    job-id-regex: Job <(\d+)>.*
    locale:
      kind: SSH
      host: hpc
    max-tasks: 10
    monitor: '~/check-job-alive ~{job_id}'
    monitor_frequency: 5
    kill: 'bkill ~{job_id}'
    shell: bash
    submit: |2-
          bsub
              -q compbio
              -n ~{cpu}
              -cwd ~{cwd}
              -o ~{cwd}/stdout.lsf
              -e ~{cwd}/stderr.lsf
              -R "rusage[mem=~{ram_mb}] span[hosts=~{hosts}]"
              ~{shell}
"#;

/// Starting point for task execution.
async fn run(args: Args) -> Result<()> {
    let config = serde_yaml::from_str::<Config>(CONFIG)
        .context("parsing LSF configuration file")?
        .into_backends()
        .find(|backend| backend.name() == "lsf")
        .context("locating configuration with name `lsf`")?;

    let engine = Engine::default().with(config).await?;

    let task = Task::builder()
        .name("my-example-task")
        .description("a longer description")
        .extend_executions(vec![
            Execution::builder()
                .working_directory(".")
                .image("ubuntu")
                .args(&[String::from("echo"), String::from("'hello, world!'")])
                .try_build()
                .unwrap(),
        ])
        .try_build()
        .unwrap();

    let receivers = (0..args.n_jobs)
        .map(|_| engine.submit("lsf", task.clone()).callback)
        .collect::<Vec<_>>();

    engine.run().await;

    for rx in receivers {
        info!(reply = ?rx.await.unwrap());
    }

    Ok(())
}

/// The main function.
fn main() -> Result<()> {
    let args = Args::parse();

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(run(args))
}
