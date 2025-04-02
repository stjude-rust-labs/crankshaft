//! A command line tool to test the [`crankshaft_docker`] crate.
//!
//! This binary will typically only be useful to developers of this crate.
#![allow(missing_docs)]
#![allow(clippy::missing_docs_in_private_items)]

use clap::Parser;
use clap::Subcommand;
use clap_verbosity_flag::Verbosity;
use crankshaft_docker::Container;
use crankshaft_docker::Docker;
use eyre::Result;
use eyre::eyre;
use tracing_log::AsTrace;
use tracing_subscriber::EnvFilter;

#[derive(clap::Parser)]
struct Args {
    #[command(subcommand)]
    command: Command,

    #[command(flatten)]
    verbose: Verbosity,
}

#[derive(Subcommand)]
enum Command {
    /// Creates a container.
    CreateContainer {
        /// The name of the image.
        image: String,

        /// The name of the container.
        name: String,

        #[arg(short, long, default_value = "latest")]
        /// The tag for the image.
        tag: String,
    },
    /// Runs a container with a particular command and prints the result.
    RunContainer {
        /// The name of the image.
        image: String,

        /// The name of the container.
        name: String,

        /// The command to run.
        command: String,

        #[arg(short, long, default_value = "latest")]
        /// The tag for the image.
        tag: String,
    },
    /// Removes a container.
    RemoveContainer {
        /// The name of the container.
        name: String,

        /// Whether or not to force the removal of the container.
        #[arg(short, long, default_value_t = false)]
        force: bool,
    },
    /// Ensures an image is stored (either by pulling it or it already
    /// existing).
    EnsureImage {
        /// The name of the image.
        ///
        /// If a tag is not specified, a default tag of `latest` is used.
        image: String,
    },

    /// Lists all images.
    ListImages,

    /// Removes an image.
    RemoveImage {
        /// The name of the image.
        image: String,

        #[arg(short, long, default_value = "latest")]
        /// The tag for the image.
        tag: String,
    },

    /// Removes all images.
    RemoveAllImages,
}

async fn create_container(
    docker: Docker,
    image: impl AsRef<str>,
    tag: impl AsRef<str>,
    name: impl Into<String>,
    program: impl Into<String>,
    args: impl IntoIterator<Item = impl Into<String>>,
) -> Result<Container> {
    Ok(docker
        .container_builder()
        .image(format!(
            "{image}:{tag}",
            image = image.as_ref(),
            tag = tag.as_ref()
        ))
        .program(program)
        .args(args)
        .attach_stdout()
        .attach_stderr()
        .try_build(name)
        .await?)
}

async fn run(args: Args) -> Result<()> {
    let docker = Docker::with_defaults().unwrap();

    match args.command {
        Command::CreateContainer { image, name, tag } => {
            create_container(
                docker,
                image,
                tag,
                name,
                "/usr/bin/env",
                ["bash", "-c", "echo 'hello, world!'"],
            )
            .await?;
        }
        Command::RunContainer {
            image,
            name,
            command,
            tag,
        } => {
            let mut command =
                shlex::split(&command).ok_or_else(|| eyre!("invalid command `{command}`"))?;
            let args = command.split_off(1);

            let container =
                create_container(docker, image, tag, name, command.remove(0), args).await?;
            let output = container.run(|| {}).await?;

            println!("exit code: {}", output.status);
            println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
            println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        }
        Command::RemoveContainer { name, force } => {
            // NOTE: `attach` is hardcoded to `true` here, but that doesn't
            // matter, because the `attach` field is never used in this call.
            let container = docker.container_from_name(name, true, true);

            if force {
                container.force_remove().await?;
            } else {
                container.remove().await?;
            }
        }
        Command::EnsureImage { image } => {
            docker.ensure_image(image).await?;
        }
        Command::ListImages => {
            docker.list_images().await?;
        }
        Command::RemoveImage { image, tag } => {
            docker.remove_image(image, tag).await?;
        }
        Command::RemoveAllImages => {
            docker.remove_all_images().await?;
        }
    };

    Ok(())
}

pub fn main() -> Result<()> {
    let args = Args::parse();

    match std::env::var("RUST_LOG") {
        Ok(_) => tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .init(),
        Err(_) => tracing_subscriber::fmt()
            .with_max_level(args.verbose.log_level_filter().as_trace())
            .init(),
    };

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(run(args))
}
