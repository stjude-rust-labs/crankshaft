//! An example for runner a task using the Docker backend service.
//!
//! You can run this command with the following command:
//!
//! `cargo run --release --example docker`

use std::env::current_dir;
use std::time::Duration;

use clap::Parser;
use crankshaft::Engine;
use crankshaft::config::backend::Kind;
use crankshaft::config::backend::docker::Config;
use crankshaft::engine::Task;
use crankshaft::engine::task::Execution;
use eyre::Context;
use eyre::Result;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use indicatif::ProgressBar;
use indicatif::ProgressStyle;
use nonempty::NonEmpty;
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
        .kind(Kind::Docker(Config::builder().build()))
        .max_tasks(args.max_tasks)
        .build();

    let engine = Engine::default()
        .with(config)
        .await
        .context("initializing Docker backend")?;

    let task = Task::builder()
        .description("a longer description")
        .executions(NonEmpty::new(
            Execution::builder()
                .work_dir(
                    current_dir()
                        .expect("a current working directory")
                        .display()
                        .to_string(),
                )
                .image("ubuntu")
                .program("echo")
                .args([String::from("hello, world!")])
                .build(),
        ))
        .build();

    let mut tasks = (0..args.n_jobs)
        .map(|_| Ok(engine.spawn("docker", task.clone())?.wait()))
        .collect::<Result<FuturesUnordered<_>>>()?;

    let progress = ProgressBar::new(tasks.len() as u64);
    progress.set_style(
        ProgressStyle::with_template(
            "{spinner:.cyan/blue} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos:>7}/{len:7} \
             {msg}",
        )
        .unwrap()
        .progress_chars("#>-"),
    );

    progress.enable_steady_tick(Duration::from_millis(100));

    let mut results = Vec::new();
    while let Some(result) = tasks.next().await {
        let failed = result.is_err();
        results.push(result);

        progress.set_message(format!(
            "task #{num} {status}",
            num = results.len(),
            status = if failed { "failed" } else { "completed" }
        ));
        progress.inc(1);
    }

    drop(progress);

    for (i, result) in results.into_iter().enumerate() {
        match result {
            Ok(output) => println!(
                "task #{num} {status}, stdout: {stdout:?}, stderr: {stderr:?}",
                num = i + 1,
                status = output.first().status,
                stdout = std::str::from_utf8(&output.first().stdout).unwrap_or("<not UTF-8>"),
                stderr = std::str::from_utf8(&output.first().stderr).unwrap_or("<not UTF-8>")
            ),
            Err(e) => println!("task #{num} failed: {e:#}", num = i + 1),
        }
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
