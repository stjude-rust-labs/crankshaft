use std::collections::HashMap;
use std::error::Error;
use std::time::Duration;

use crankshaft_monitor::proto::Event;
use crankshaft_monitor::proto::GetServerStateRequest;
use crankshaft_monitor::proto::Resources;
use crankshaft_monitor::proto::SubscribeEventsRequest;
use crankshaft_monitor::proto::monitor_client::MonitorClient;
use futures_util::StreamExt;
use tokio::net::UnixStream;
use tonic::Streaming;
use tonic::transport::Channel;
use tonic::transport::Endpoint;
use tonic::transport::Uri;

/// Connection struct that holds the state of tui and Address of server
#[derive(Debug)]
pub struct Connection {
    /// the server's addr
    target: Uri,
    /// the state of tui (connected or disconnected)
    state: State,
}

/// State of the connection
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
enum State {
    /// Connected state
    Connected {
        /// The client connection to the server
        _client: MonitorClient<Channel>,
        /// The stream of events from the server
        update_stream: Box<Streaming<Event>>,
        /// The tasks currently running on the server
        _tasks: HashMap<String, i32>,
        /// The resources currently available on the server
        _resources: Option<Resources>,
    },
    /// Disconnected state
    Disconnected(Duration),
}

impl Connection {
    /// backoff duration
    const BACKOFF: Duration = Duration::from_millis(500);

    /// create a new connection
    pub fn new(target: Uri) -> Self {
        Self {
            target,
            state: State::Disconnected(Duration::from_secs(0)),
        }
    }

    /// connect to the server
    async fn connect(&mut self) {
        /// max backoff duration
        const MAX_BACKOFF: Duration = Duration::from_secs(5);

        while let State::Disconnected(backoff) = self.state {
            if backoff > Duration::from_secs(0) {
                tokio::time::sleep(backoff).await;
            }
            let try_connect = async {
                let channel = match self.target.scheme_str() {
                    #[cfg(unix)]
                    Some("file") => {
                        use tonic::transport::Endpoint;

                        if !matches!(self.target.host(), None | Some("localhost")) {
                            return Err("cannot connect to non-localhost unix domain socket".into());
                        }
                        let path = self.target.path().to_owned();
                        let endpoint = Endpoint::from_static("http://localhost");
                        endpoint
                            .connect_with_connector(tower::service_fn(move |_| {
                                use futures_util::TryFutureExt;
                                use hyper_util::rt::TokioIo;

                                UnixStream::connect(path.clone()).map_ok(TokioIo::new)
                            }))
                            .await?
                    }
                    #[cfg(feature = "vsock")]
                    Some("vsock") => {
                        if !matches!(self.target.host(), None | Some("localhost") | Some("any")) {
                            return Err("cannot connect to non-localhost vsock".into());
                        }

                        // Parse URI path in the format vsock://<cid>:<port>
                        let uri_path = self.target.path();
                        let parts: Vec<&str> =
                            uri_path.trim_start_matches('/').split(':').collect();
                        if parts.len() != 2 {
                            return Err(format!(
                                "invalid vsock URI format, expected vsock://<cid>:<port>, got {}",
                                self.target
                            )
                            .into());
                        }

                        let cid = parts[0]
                            .parse::<u32>()
                            .map_err(|_| format!("invalid CID: {}", parts[0]))?;
                        let port = parts[1]
                            .parse::<u32>()
                            .map_err(|_| format!("invalid port: {}", parts[1]))?;

                        let vsock_addr = tokio_vsock::VsockAddr::new(cid, port);

                        // Dummy endpoint is ignored by the connector
                        let endpoint = Endpoint::from_static("http://localhost");
                        endpoint
                            .connect_with_connector(tower::service_fn(move |_| {
                                VsockStream::connect(vsock_addr).map_ok(TokioIo::new)
                            }))
                            .await?
                    }
                    #[cfg(not(feature = "vsock"))]
                    Some("vsock") => {
                        return Err("vsock feature is not enabled".into());
                    }
                    _ => {
                        let endpoint = Endpoint::from(self.target.clone());
                        endpoint.connect().await?
                    }
                    #[cfg(not(unix))]
                    Some("file") => {
                        return Err("unix domain sockets are not supported on this platform".into());
                    }
                };
                let mut client = MonitorClient::new(channel);
                let update_request = tonic::Request::new(SubscribeEventsRequest {});
                let state_request = tonic::Request::new(GetServerStateRequest {});

                let update_stream =
                    Box::new(client.subscribe_events(update_request).await?.into_inner());

                let state = client.get_server_state(state_request).await?.into_inner();
                let tasks = state.tasks;
                // we know that resources is there , how to safley exit this option , this
                // option is due to proto
                let resources = state.resources;

                Ok::<State, Box<dyn Error + Send + Sync>>(State::Connected {
                    _client: client,
                    update_stream,
                    _tasks: tasks,
                    _resources: resources,
                })
            };
            self.state = match try_connect.await {
                Ok(connected) => connected,
                Err(_error) => {
                    let backoff = std::cmp::max(backoff + Self::BACKOFF, MAX_BACKOFF);
                    State::Disconnected(backoff)
                }
            };
        }
    }

    /// next message
    pub async fn next_message(&mut self) -> Event {
        loop {
            match &mut self.state {
                State::Connected { update_stream, .. } => match update_stream.next().await {
                    Some(Ok(update)) => return update,
                    Some(Err(_status)) => {
                        self.state = State::Disconnected(Self::BACKOFF);
                    }
                    None => {
                        self.state = State::Disconnected(Self::BACKOFF);
                    }
                },
                State::Disconnected(_) => self.connect().await,
            }
        }
    }

    pub fn initial_state(&self) -> Option<(&HashMap<String, i32>, &Option<Resources>)> {
        match &self.state {
            State::Connected {
                _tasks, _resources, ..
            } => Some((_tasks, _resources)),
            _ => None,
        }
    }

    /// render
    pub fn render(&self, styles: &crate::view::styles::Styles) -> ratatui::text::Line<'_> {
        use ratatui::style::Color;
        use ratatui::style::Modifier;
        use ratatui::text::Line;
        use ratatui::text::Span;
        let state = match self.state {
            State::Connected { .. } => Span::styled(
                "(CONNECTED)",
                styles.fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
            State::Disconnected(d) if d == Duration::from_secs(0) => Span::styled(
                "(CONNECTING)",
                styles.fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            State::Disconnected(d) => Span::styled(
                format!("(RECONNECTING IN {d:?})"),
                styles.fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
        };
        Line::from(vec![
            Span::raw("connection: "),
            Span::raw(self.target.to_string()),
            Span::raw(" "),
            state,
        ])
    }
}
