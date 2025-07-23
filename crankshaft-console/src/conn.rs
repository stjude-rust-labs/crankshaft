//! The module for handling connections to the crankshaft server.
use std::error::Error;
use std::time::Duration;

use crankshaft_monitor::proto::Event;
use crankshaft_monitor::proto::GetServerStateRequest;
use crankshaft_monitor::proto::SubscribeEventsRequest;
use crankshaft_monitor::proto::monitor_client::MonitorClient;
use futures_util::StreamExt;
use tokio::net::UnixStream;
use tonic::Streaming;
use tonic::transport::Channel;
use tonic::transport::Endpoint;
use tonic::transport::Uri;

use crate::state::State as TuiState;

/// exponential backoff duration
const BACKOFF: Duration = Duration::from_millis(500);
/// maximum backoff duration
const MAX_BACKOFF: Duration = Duration::from_secs(5);

/// Connection struct that holds the state of tui and Address of server
#[derive(Debug)]
pub struct Connection {
    /// the server's addr
    target: Uri,
    /// the state of tui (connected or disconnected)
    state: ConnectionState,
}

/// State of the connection
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
enum ConnectionState {
    /// Connected state
    Connected {
        /// The client connection to the server
        _client: MonitorClient<Channel>,
        /// The stream of events from the server
        update_stream: Box<Streaming<Event>>,
    },
    /// Disconnected state
    Disconnected(Duration),
}

impl Connection {
    /// create a new connection
    pub fn new(target: Uri) -> Self {
        Self {
            target,
            state: ConnectionState::Disconnected(Duration::from_secs(0)),
        }
    }

    /// connect to the server
    pub async fn connect(&mut self, state: &mut TuiState) {
        while let ConnectionState::Disconnected(backoff) = self.state {
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

                let server_state = client.get_server_state(state_request).await?.into_inner();
                let tasks = server_state.tasks;
                let resources = server_state.resources;
                state.set_initial_state(tasks, resources);

                Ok::<ConnectionState, Box<dyn Error + Send + Sync>>(ConnectionState::Connected {
                    _client: client,
                    update_stream,
                })
            };
            self.state = match try_connect.await {
                Ok(connected) => connected,
                Err(_error) => {
                    let backoff = std::cmp::max(backoff + BACKOFF, MAX_BACKOFF);
                    ConnectionState::Disconnected(backoff)
                }
            };
        }
    }

    /// next message
    pub async fn next_message(&mut self, state: &mut TuiState) -> Event {
        loop {
            match &mut self.state {
                ConnectionState::Connected { update_stream, .. } => {
                    match update_stream.next().await {
                        Some(Ok(update)) => return update,
                        Some(Err(_status)) => {
                            self.state = ConnectionState::Disconnected(BACKOFF);
                        }
                        None => {
                            self.state = ConnectionState::Disconnected(BACKOFF);
                        }
                    }
                }
                ConnectionState::Disconnected(_) => self.connect(state).await,
            }
        }
    }

    /// render
    pub fn render(&self, styles: &crate::view::styles::Styles) -> ratatui::text::Line<'_> {
        use ratatui::style::Color;
        use ratatui::style::Modifier;
        use ratatui::text::Line;
        use ratatui::text::Span;
        let state = match self.state {
            ConnectionState::Connected { .. } => Span::styled(
                "(CONNECTED)",
                styles.fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
            ConnectionState::Disconnected(d) if d == Duration::from_secs(0) => Span::styled(
                "(CONNECTING)",
                styles.fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            ConnectionState::Disconnected(d) => Span::styled(
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
