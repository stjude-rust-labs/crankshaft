//! An example for runner a task using the Docker backend service.
//!
//! You can run this command with the following command:
//!
//! `cargo run --release --example docker`

use std::io::Write;

use clap::Parser;
use crankshaft::config::backend::docker::Config;
use crankshaft::config::backend::Kind;
use crankshaft::engine::task::input;
use crankshaft::engine::task::Execution;
use crankshaft::engine::task::Input;
use crankshaft::engine::Task;
use crankshaft::Engine;
use eyre::Context;
use eyre::Result;
use tempfile::NamedTempFile;
use tracing::info;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::util::SubscriberInitExt as _;
use tracing_subscriber::EnvFilter;

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
        .kind(Kind::Docker(Config::builder().cleanup(true).build()))
        .max_tasks(args.max_tasks)
        .try_build()
        .context("building backend configuration")?;

    let engine = Engine::default()
        .with(config)
        .await
        .context("initializing Docker backend")?;

    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "Hello, world from an input").unwrap();

    // Get the path to the temp file
    let temp_path = temp_file.path().to_path_buf();
    let input = Input::builder()
        .contents(temp_path)
        .path("/volA/test_input.txt")
        .r#type(input::Type::File)
        .try_build()
        .unwrap();

    let task = Task::builder()
        .extend_inputs([input])
        .extend_executions([
            Execution::builder()
                .image("ubuntu")
                .args(&[
                    String::from("bash"),
                    String::from("-c"),
                    String::from("ls /volA"),
                ])
                .try_build()
                .unwrap(),
            Execution::builder()
                .image("ubuntu")
                .args(&[String::from("cat"), String::from("/volA/test_input.txt")])
                .try_build()
                .unwrap(),
        ])
        .extend_volumes([String::from("/volA"), String::from("/volB")])
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
