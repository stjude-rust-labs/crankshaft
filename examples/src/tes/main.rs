//! An example for runner a task using a TES backend service available at a
//! remote URL.
//!
//! You can run this command with the following command:
//! ```bash
//! export USER="<USER>"
//! export PASSWORD="<PASSWORD>"
//!
//! cargo run --release --example tes <URL>
//! ```

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use clap::Parser;
use crankshaft::Engine;
use crankshaft::config::backend::Kind;
use crankshaft::config::backend::tes::Config;
use crankshaft::config::backend::tes::http;
use crankshaft::engine::Task;
use crankshaft::engine::task::Execution;
use eyre::Context;
use eyre::Result;
use tracing::info;
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
async fn run(args: Args) -> Result<()> {
    let mut config = Config::builder()
        .url(args.url)
        .http(http::Config::default());

    let username = std::env::var(USER_ENV).ok();
    let password = std::env::var(PASSWORD_ENV).ok();

    if (username.is_some() && password.is_none()) || (username.is_none() && password.is_some()) {
        panic!("both username and password must be provided for authentication");
    }

    // If username and password are available, add them to the config.
    if let (Some(username), Some(password)) = (username, password) {
        let credentials = format!("{}:{}", username, password);
        let token = STANDARD.encode(credentials);
        config = config.basic_auth_token(token);
    }

    let config = crankshaft::config::backend::Config::builder()
        .name("tes")
        .kind(Kind::TES(
            config
                .try_build()
                .context("building TES backend configuration")?,
        ))
        .max_tasks(args.max_tasks)
        .try_build()
        .context("building backend configuration")?;

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
        .map(|_| engine.submit("tes", task.clone()).callback)
        .collect::<Vec<_>>();

    #[cfg(tokio_unstable)]
    Engine::start_instrument(3000);

    engine.run().await;

    for rx in receivers {
        info!(reply = ?rx.await.unwrap());
    }

    Ok(())
}

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
