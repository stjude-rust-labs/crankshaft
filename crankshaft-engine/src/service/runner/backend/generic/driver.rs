//! Command drivers in a generic backend.

use std::io::Read as _;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
#[cfg(windows)]
use std::os::windows::process::ExitStatusExt;
use std::process::ExitStatus;
use std::process::Output;
use std::sync::Arc;
use std::time::Duration;

use crankshaft_config::backend::generic::driver::Config;
use crankshaft_config::backend::generic::driver::Locale;
use crankshaft_config::backend::generic::driver::Shell;
use crankshaft_config::backend::generic::driver::ssh;
use eyre::Context as _;
use eyre::Result;
use eyre::bail;
use rand::Rng as _;
use ssh2::Channel;
use ssh2::Session;
use thiserror::Error;
use tokio::net::TcpStream;
use tokio::process::Command;
use tracing::debug;
use tracing::error;
use tracing::trace;

/// An error related to a [`Driver`].
#[derive(Error, Debug)]
pub enum Error {
    /// An i/o error.
    #[error(transparent)]
    Io(std::io::Error),

    /// An error related to joining a [`tokio`] task.
    #[error(transparent)]
    Join(tokio::task::JoinError),

    /// An [ssh error](ssh2::Error).
    #[error(transparent)]
    SSH2(ssh2::Error),
}

/// A command transport.
///
/// The command transport is what ships commands off to be run within an
/// [`Driver`]. This might be executing commands locally or on a remote server
/// via SSH.
pub enum Transport {
    /// Local command execution.
    Local,

    /// Command execution over an SSH session.
    SSH(Arc<Session>),
}

impl std::fmt::Debug for Transport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Local => write!(f, "Local"),
            Self::SSH(_) => f.debug_tuple("SSH").finish(),
        }
    }
}

/// A command driver.
///
/// A command driver is an abstraction through which shell commands can be
/// dispatched within various locales (e.g., your local computer, remotely over
/// SSH, etc).
///
/// In addition to containing the state around those connections, the driver
/// also holds configuration necessary to know how to execute the commands
/// (e.g., which shell to use when running).
#[derive(Debug)]
pub struct Driver {
    /// The command transport.
    transport: Transport,

    /// The configuration.
    config: Config,
}

impl Driver {
    /// Initializes a new [`Driver`].
    ///
    /// This command requires an async runtime because, for some transports,
    /// negotiation is done via subprocesses or network calls to initialize the
    /// necessary state (e.g., establishing an SSH session with a remote host).
    ///
    /// **NOTE:** this method returns an [`eyre::Result`] because any errors
    /// are intended to be returned directly to the user in the calling binary
    /// (i.e., the errors are typically unrecoverable).
    pub async fn initialize(config: Config) -> Result<Self> {
        // NOTE: this is cloned because `default()` is only implemented on the
        // owned [`Locale`] type (not a reference).
        let transport = match config.locale().cloned().unwrap_or_default() {
            // NOTE: no initialization is needed here, as we simply spawn a
            // [`tokio::process::Command`] when [`command()`] is called.
            Locale::Local => Ok(Transport::Local),
            Locale::SSH { host, options } => create_ssh_transport(&host, &options).await,
        }?;

        Ok(Self { transport, config })
    }

    /// Runs a shell command within the configuration locale.
    ///
    /// **NOTE:** this method returns an [`eyre::Result`] because any errors
    /// are intended to be returned directly to the user in the calling binary
    /// (i.e., the errors are typically unrecoverable).
    pub async fn run(&self, command: impl Into<String>) -> Result<Output> {
        let command = command.into();

        match &self.transport {
            Transport::Local => run_local_command(command, &self.config).await,
            Transport::SSH(session) => {
                run_ssh_command(session.clone(), &self.config, command).await
            }
        }
    }

    /// Gets the inner transport.
    pub fn transport(&self) -> &Transport {
        &self.transport
    }

    /// Gets the inner config.
    pub fn config(&self) -> &Config {
        &self.config
    }
}

//=================//
// Local Execution //
//=================//

/// Runs a command in a local context.
async fn run_local_command(command: String, config: &Config) -> Result<Output> {
    trace!("executing local command: `{command}`");

    // NOTE: this is cloned because `default()` is only implemented on the owned
    // [`Locale`] type (not a reference).
    let command = match config.shell().cloned().unwrap_or_default() {
        Shell::Bash => Command::new("/usr/bin/env")
            .args(["bash", "-c", &command])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn(),
        Shell::Sh => Command::new("/usr/bin/env")
            .args(["sh", "-c", &command])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn(),
    }
    .context("spawning the local command")?;

    command
        .wait_with_output()
        .await
        .context("executing the local command")
}

//===============//
// SSH Execution //
//===============//

/// Attempts to create an SSH transport.
async fn create_ssh_transport(host: &str, config: &ssh::Config) -> Result<Transport> {
    let addr = format!("{host}:{}", config.port());

    // Connect to the remote SSH host.
    let message = format!("connecting to SSH host: {}", addr);
    debug!(message);
    let tcp = TcpStream::connect(addr)
        .await
        .map_err(Error::Io)
        .context(message)?;

    // Create a new SSH session.
    debug!("establishing a new SSH session and connecting");

    trace!("creating a new SSH session");
    let mut sess = Session::new()
        .map_err(Error::SSH2)
        .context("creating a new SSH session")?;
    sess.set_tcp_stream(tcp);
    trace!("performing the SSH handshake");
    sess.handshake()
        .map_err(Error::SSH2)
        .context("performing the SSH handshake")?;

    // Connect to the SSH agent and authenticate within the current
    // session.
    debug!("retrieving identities from the SSH agent");

    trace!("initializing the SSH agent");
    let mut agent = sess
        .agent()
        .map_err(Error::SSH2)
        .context("initializing the SSH agent")?;

    trace!("connecting to the SSH agent");
    agent
        .connect()
        .map_err(Error::SSH2)
        .context("connecting to the SSH agent")?;

    trace!("listing identities within the SSH agent");
    agent
        .list_identities()
        .map_err(Error::SSH2)
        .context("listing identities within the SSH agent")?;

    trace!("accessing the retrieved identities");
    let identities = agent
        .identities()
        .map_err(Error::SSH2)
        .context("accessing retrieved identities")?;

    let key = match identities.len() {
        0 => bail!("no identities found in the SSH agent! Try using `ssh-add` on your SSH key."),
        // SAFETY: we just checked that there is exactly one SSH key
        // in the agent, so this will always unwrap.
        1 => identities.first().unwrap(),
        _ => unimplemented!(
            "`crankshaft` does not yet support multiple keys in an SSH agent. Please file an \
             issue!"
        ),
    };

    trace!(
        "found a single identifier with the comment `{}`",
        key.comment()
    );

    // Authenticate the SSH session.
    debug!("authenticating SSH session");

    if let Some(ref username) = config.username() {
        agent
            .userauth(username, key)
            .map_err(Error::SSH2)
            .with_context(|| {
                format!(
                    "authenticating with username `{}` and identity `{}`",
                    username,
                    key.comment()
                )
            })?;
    } else {
        let username = whoami::username();

        agent
            .userauth(&username, key)
            .map_err(Error::SSH2)
            .with_context(|| {
                format!(
                    "authenticating with username `{}` and identity `{}`",
                    username,
                    key.comment()
                )
            })?;
    }

    if sess.authenticated() {
        debug!("authentication successful");
        Ok(Transport::SSH(Arc::new(sess)))
    } else {
        error!("authentication failed!");
        bail!("failed authentication")
    }
}

/// The minimum amount of waiting time.
const WAIT_FLOOR: u64 = 300;

/// The amount of jitter to introduce.
const WAIT_JITTER: u64 = 150;

/// Attempts to create a new [`Channel`] with a backoff on failures.
fn channel_session_with_backoff(
    session: &Session,
    max_attempts: u32,
) -> std::result::Result<Channel, Error> {
    let mut attempts = 0u32;
    let mut wait_time = 0u64;

    while attempts < max_attempts {
        match session.channel_session() {
            Ok(channel) => return Ok(channel),
            Err(e) => {
                attempts += 1;
                trace!(
                    "failed to connect: {}; attempt {}/{}",
                    e, attempts, max_attempts,
                );

                if attempts >= max_attempts {
                    return Err(Error::SSH2(e));
                }

                let jitter = rand::thread_rng().gen_range(0..=WAIT_JITTER);
                wait_time += WAIT_FLOOR + jitter;

                trace!("waiting for {} ms.", wait_time);
                // NOTE: this will always be called from a blocking thread in
                // the async runtime, so it's okay.
                std::thread::sleep(Duration::from_millis(wait_time));
            }
        }
    }

    // SAFETY: the loop above should always return.
    unreachable!()
}

/// Runs a remote command over SSH.
async fn run_ssh_command(
    session: Arc<ssh2::Session>,
    config: &Config,
    command: String,
) -> Result<Output> {
    let max_attempts = config.max_attempts();

    let f = move || {
        debug!("running command on remote host: `{}`", command);

        // Create a new channel with which to communicate with the host.
        trace!("creating a new session-based channel");
        let mut channel = channel_session_with_backoff(&session, max_attempts)
            .context("creating a new session-based channel")?;

        // Send a command across the channel.
        trace!("sending the execution command");
        channel
            .exec(&command)
            .map_err(Error::SSH2)
            .context("executing a command over SSH")?;

        // Read the entire output that was written to the channel.
        trace!("reading the stdout of the command");
        let mut stdout = Vec::new();
        channel
            .read_to_end(&mut stdout)
            .map_err(Error::Io)
            .context("reading the stdout of the command over SSH")?;

        for line in String::from_utf8_lossy(&stdout).lines() {
            trace!("stdout: {line}");
        }

        // Read the entire stderr that was written to the channel.
        trace!("reading the stderr of the command");
        let mut stderr = Vec::new();
        channel
            .stderr()
            .read_to_end(&mut stderr)
            .map_err(Error::Io)
            .context("reading the stderr of the command over SSH")?;

        for line in String::from_utf8_lossy(&stderr).lines() {
            trace!("stderr: {line}");
        }

        // Getting the exit code.
        let status = channel
            .exit_status()
            .map_err(Error::SSH2)
            .context("getting the exit status of the command")?;

        // Indicate to the remote host that we won't be sending any
        // more data over this connection.
        trace!("closing the client's end of the channel");
        channel
            .close()
            .map_err(Error::SSH2)
            .context("closing the SSH channel")?;

        // Wait until the remote host also closes the connection.
        trace!("waiting for the remote host to close their end of the channel");
        channel
            .wait_close()
            .map_err(Error::SSH2)
            .context("waiting for the SSH channel to be closed from the client's end")?;

        #[cfg(unix)]
        let output = Output {
            status: ExitStatus::from_raw(status),
            stdout,
            stderr,
        };

        #[cfg(windows)]
        let output = Output {
            status: ExitStatus::from_raw(status as u32),
            stdout,
            stderr,
        };

        eyre::Result::<Output>::Ok(output)
    };

    tokio::task::spawn_blocking(f)
        .await
        .map_err(Error::Join)
        .context("running an SSH command")?
}
