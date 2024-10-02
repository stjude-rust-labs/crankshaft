//! An example for runner a task using the Docker backend service.
//!
//! You can run this command with the following command:
//!
//! `cargo run --release --example docker`

use std::env::current_dir;

use clap::Parser;
use crankshaft::Engine;
use crankshaft::config::backend::Kind;
use crankshaft::config::backend::docker::Config;
use crankshaft::engine::Task;
use crankshaft::engine::task::Execution;
use eyre::Context;
use eyre::Result;
use tracing::info;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::util::SubscriberInitExt as _;

#[derive(Debug, Parser)]
#[allow(missing_docs)]
pub struct Args {
    /// The maximum number of concurrent tasks.
    #[arg(short, long, default_value_t = 50)]
    max_tasks: usize,

    /// The number of jobs to submit in total.
    #[arg(short, long, default_value_t = 1000)]
    n_jobs: usize,
}

/// Starting point for task execution.
async fn run(args: Args) -> Result<()> {
    let config = crankshaft::config::backend::Config::builder()
        .name("docker")
        .kind(Kind::Docker(Config::builder().cleanup(false).build()))
        .max_tasks(args.max_tasks)
        .try_build()
        .context("building backend configuration")?;

    let engine = Engine::default()
        .with(config)
        .await
        .context("initializing Docker backend")?;

    let task = Task::builder()
        .description("a longer description")
        .extend_executions(vec![
            Execution::builder()
                .working_directory(
                    current_dir()
                        .expect("a current working directory")
                        .display()
                        .to_string(),
                )
                .image("ubuntu")
                .args(&[String::from("echo"), String::from("'hello, world!'")])
                .try_build()
                .unwrap(),
        ])
        .try_build()
        .unwrap();

    let receivers = (0..args.n_jobs)
        .map(|_| engine.submit("docker", task.clone()).callback)
        .collect::<Vec<_>>();

    engine.run().await;

    for rx in receivers {
        info!(runner = "Docker", reply = ?rx.await.unwrap());
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
