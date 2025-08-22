//! An example for runner a task using the Docker backend service.
//!
//! You can run this command with the following command:
//!
//! `cargo run --release --bin docker`

use std::env::current_dir;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use anyhow::anyhow;
use clap::Parser;
use crankshaft::Engine;
use crankshaft::config::backend::Kind;
use crankshaft::config::backend::docker::Config;
use crankshaft::engine::Task;
use crankshaft::engine::task::Execution;
use crankshaft::engine::task::Output;
use crankshaft::engine::task::output::Type;
use crankshaft_monitor::proto::SubscribeEventsRequest;
use crankshaft_monitor::proto::monitor_client::MonitorClient;
use futures::FutureExt;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use nonempty::NonEmpty;
use tempfile::NamedTempFile;
use tokio::select;
use tokio::signal;
use tokio_util::sync::CancellationToken;
use tonic::Request;
use tonic::transport::Channel;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::util::SubscriberInitExt as _;
use url::Url;

#[derive(Debug, Parser)]
#[allow(missing_docs)]
pub struct Args {
    /// The maximum number of concurrent tasks.
    #[arg(short, long, default_value_t = 50)]
    max_tasks: usize,

    /// The number of jobs to submit in total.
    #[arg(short, long, default_value_t = 10)]
    n_jobs: usize,
}

/// Starting point for task execution.
async fn run(args: Args, token: CancellationToken) -> Result<()> {
    let config = crankshaft::config::backend::Config::builder()
        .name("docker")
        .kind(Kind::Docker(Config::builder().build()))
        .max_tasks(args.max_tasks)
        .build();

    let engine = Engine::new_with_monitoring("127.0.0.1:8080".parse().unwrap())
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
                .image("alpine")
                .program("sh")
                .args([
                    String::from("-c"),
                    String::from("for i in $(seq 100); do echo hello_world; sleep 1; done"),
                ])
                .stdout("/stdout")
                .stderr("/stderr")
                .build(),
        ))
        .build();

    let mut did_start_polling = false;
    let mut tasks = FuturesUnordered::new();

    for i in 0..args.n_jobs {
        let mut task = task.clone();
        let stdout = NamedTempFile::new()?.into_temp_path();
        let stderr = NamedTempFile::new()?.into_temp_path();

        task.override_name(format!("task {i}"));
        task.add_output(
            Output::builder()
                .path("/stdout")
                .url(
                    Url::from_file_path(&stdout)
                        .map_err(|_| anyhow!("failed to get stdout URL"))?,
                )
                .ty(Type::File)
                .build(),
        );
        task.add_output(
            Output::builder()
                .path("/stderr")
                .url(
                    Url::from_file_path(&stderr)
                        .map_err(|_| anyhow!("failed to get stderr URL"))?,
                )
                .ty(Type::File)
                .build(),
        );

        let handle = engine.spawn("docker", task, token.clone())?;

        if !did_start_polling {
            tokio::time::sleep(Duration::from_millis(1)).await;
            start_polling();
            did_start_polling = true;
        }

        tasks.push(
            handle
                .wait()
                .map(|e| e.map(|e| (e.into_iter().next().unwrap(), stdout, stderr))),
        );
    }

    let mut results = Vec::new();
    while let Some(result) = tasks.next().await {
        let _failed = result.is_err();
        results.push(result);
    }

    for (i, result) in results.into_iter().enumerate() {
        match result {
            Ok((status, stdout, stderr)) => println!(
                "task #{num} {status}, stdout: {stdout:?}, stderr: {stderr:?}",
                num = i + 1,
                stdout = std::fs::read_to_string(&stdout).with_context(|| format!(
                    "failed to read stdout file `{stdout}`",
                    stdout = stdout.display()
                ))?,
                stderr = std::fs::read_to_string(&stderr).with_context(|| format!(
                    "failed to read stderr file `{stderr}`",
                    stderr = stderr.display()
                ))?,
            ),
            Err(e) => println!("task #{num} failed: {e:#}", num = i + 1),
        }
    }

    Ok(())
}

/// performs polling
fn start_polling() {
    tokio::spawn(async {
        let addr = "http://127.0.0.1:8080";
        let channel = match Channel::from_static(addr).connect().await {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to connect to monitor server: {e:#}");
                return;
            }
        };

        let mut client = MonitorClient::new(channel);

        let request = Request::new(SubscribeEventsRequest {});

        let response = match client.subscribe_events(request).await {
            Ok(res) => {
                println!("✅ gRPC client connected and subscribed to events");
                res.into_inner()
            }
            Err(e) => {
                eprintln!("Failed to subscribe to events: {e:#}");
                return;
            }
        };

        let mut stream = response;
        while let Some(next) = stream.next().await {
            match next {
                Ok(event) => println!("{event:?}"),
                Err(e) => {
                    eprintln!("Error receiving event: {e:#}");
                    break;
                }
            }
        }
    });
}

/// The main function.
#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let cancellation = CancellationToken::new();
    let mut run = run(args, cancellation.clone()).boxed();

    select! {
        _ = signal::ctrl_c() => {
            eprintln!("\nexecution was interrupted; waiting for tasks to cancel");
            cancellation.cancel();
            run.await
        },
        res = &mut run => return res,
    }
}
