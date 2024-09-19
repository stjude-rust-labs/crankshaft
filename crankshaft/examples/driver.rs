//! An example for using remote command localization.
//!
//! You can run this command with the following command:
//!
//! `cargo run --release --example driver <HOST> <CMD>`

use std::process::Output;

use clap::Parser;
use crankshaft::config::backend::generic::driver::Config;
use crankshaft::engine::service::runner::backend::generic::driver::Driver;
use eyre::Result;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::util::SubscriberInitExt as _;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[allow(missing_docs)]
pub struct Args {
    /// The host where the commands should be run.
    host: String,

    /// The command to be run.
    command: String,

    /// The port on the remote host.
    #[arg(short, long, default_value_t = 22)]
    port: usize,
}

/// Starting point for task execution.
async fn run(args: &Args) -> Result<()> {
    let url = format!("{}:{}", args.host, args.port);

    let local = local_command(&args.command).await?;
    eprintln!("Local result: {:#?}", local);

    let remote = remote_command(&url, &args.command).await?;
    eprintln!("Remote result: {:#?}", remote);

    Ok(())
}

/// Runs a command on the local machine.
async fn local_command(command: &str) -> Result<Output> {
    let config = Config::builder().build();
    let driver = Driver::initialize(config).await.unwrap();
    driver.run(command).await
}

/// Runs a command on a remote machine via SSH.
async fn remote_command(url: &str, command: &str) -> Result<Output> {
    let config = Config::builder().with_ssh(url, Default::default()).build();
    let driver = Driver::initialize(config).await.unwrap();
    driver.run(command).await
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
        .block_on(run(&args))
}
