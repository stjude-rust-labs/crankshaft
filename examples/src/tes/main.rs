//! An example for runner a task using a TES backend service available at a
//! remote URL.
//!
//! You can run this command with the following command:
//! ```bash
//! export USER="<USER>"
//! export PASSWORD="<PASSWORD>"
//!
//! cargo run --release --bin tes <URL>
//! ```

use std::env::current_dir;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use anyhow::anyhow;
use anyhow::bail;
use clap::Parser;
use crankshaft::Engine;
use crankshaft::config::backend::Kind;
use crankshaft::config::backend::tes::Config;
use crankshaft::config::backend::tes::http;
use crankshaft::engine::Task;
use crankshaft::engine::task::Execution;
use crankshaft::engine::task::Output;
use crankshaft::engine::task::output::Type;
use futures::FutureExt;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use indicatif::ProgressBar;
use indicatif::ProgressStyle;
use nonempty::NonEmpty;
use tempfile::NamedTempFile;
use tokio::select;
use tokio::signal;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::util::SubscriberInitExt as _;
use url::Url;

/// The environment variable for a basic auth username.
const USER_ENV: &str = "USER";

/// The environment variable for a basic auth password.
const PASSWORD_ENV: &str = "PASSWORD";

#[derive(Debug, Parser)]
#[allow(missing_docs)]
pub struct Args {
    /// The URL of the TES service.
    url: Url,

    /// The maximum number of concurrent tasks.
    #[arg(short, long, default_value_t = 50)]
    max_tasks: usize,

    /// The number of jobs to submit.
    #[arg(short, long, default_value_t = 1000)]
    n_jobs: usize,
}

/// Starting point for task execution.
async fn run(args: Args, token: CancellationToken) -> Result<()> {
    let config = Config::builder().url(args.url);

    let username = std::env::var(USER_ENV).ok();
    let password = std::env::var(PASSWORD_ENV).ok();

    if (username.is_some() && password.is_none()) || (username.is_none() && password.is_some()) {
        bail!("both username and password must be provided for authentication");
    }

    let mut http_config = http::Config::default();

    // If username and password are available, add them to the config.
    if let (Some(username), Some(password)) = (username, password) {
        http_config.auth = Some(http::HttpAuthConfig::Basic { username, password });
    }

    let config = crankshaft::config::backend::Config::builder()
        .name("tes")
        .kind(Kind::TES(config.http(http_config).build()))
        .max_tasks(args.max_tasks)
        .monitoring(false)
        .build();

    let engine = Engine::default().with(config).await?;

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
                .program("echo")
                .args([String::from("hello, world!")])
                .stdout("/stdout")
                .stderr("/stderr")
                .build(),
        ))
        .build();

    #[cfg(tokio_unstable)]
    Engine::start_instrument(3000);

    let mut tasks = (0..args.n_jobs)
        .map(|_| {
            let mut task = task.clone();
            let stdout = NamedTempFile::new()?.into_temp_path();
            let stderr = NamedTempFile::new()?.into_temp_path();

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

            let handle = engine.spawn("tes", task, token.clone())?;
            Ok(handle
                .wait()
                .map(|e| e.map(|e| (e.into_iter().next().unwrap(), stdout, stderr))))
        })
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
