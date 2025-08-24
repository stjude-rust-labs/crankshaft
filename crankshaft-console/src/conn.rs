//! The `conn` module handles connections to the crankshaft server.
use std::error::Error;
use std::time::Duration;

use crankshaft_monitor::proto::Event;
use crankshaft_monitor::proto::ServiceStateRequest;
use crankshaft_monitor::proto::SubscribeEventsRequest;
use crankshaft_monitor::proto::monitor_client::MonitorClient;
use futures_util::StreamExt;
use tonic::Streaming;
use tonic::transport::Channel;
use tonic::transport::Endpoint;
use tonic::transport::Uri;

use crate::state::State as TuiState;

/// The exponential backoff duration.
const BACKOFF: Duration = Duration::from_millis(500);
/// The maximum backoff duration.
const MAX_BACKOFF: Duration = Duration::from_secs(5);

/// The `Connection` struct holds the state of the TUI and the server address.
#[derive(Debug)]
pub struct Connection {
    /// The server's address.
    target: Uri,
    /// The state of the connection.
    state: ConnectionState,
}

/// The state of the connection.
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
enum ConnectionState {
    /// The connected state.
    Connected {
        /// The client connection to the server.
        _client: MonitorClient<Channel>,
        /// The stream of events from the server.
        update_stream: Box<Streaming<Event>>,
    },
    /// The disconnected state.
    Disconnected(Duration),
}

impl Connection {
    /// Creates a new connection.
    pub fn new(target: Uri) -> Self {
        Self {
            target,
            state: ConnectionState::Disconnected(Duration::from_secs(0)),
        }
    }

    /// Connects to the server.
    pub async fn connect(&mut self, state: &mut TuiState) {
        while let ConnectionState::Disconnected(backoff) = self.state {
            if backoff > Duration::from_secs(0) {
                tokio::time::sleep(backoff).await;
            }
            let try_connect = async {
                let endpoint = Endpoint::from(self.target.clone());
                let channel = endpoint.connect().await?;
                let mut client = MonitorClient::new(channel);

                let update_stream = Box::new(
                    client
                        .subscribe_events(SubscribeEventsRequest {})
                        .await?
                        .into_inner(),
                );

                let service_state = client
                    .get_service_state(ServiceStateRequest {})
                    .await?
                    .into_inner();
                state.set_initial_state(service_state);

                Ok::<ConnectionState, Box<dyn Error + Send + Sync>>(ConnectionState::Connected {
                    _client: client,
                    update_stream,
                })
            };
            self.state = match try_connect.await {
                Ok(connected) => connected,
                Err(error) => {
                    let backoff = std::cmp::max(backoff + BACKOFF, MAX_BACKOFF);
                    tracing::warn!(
                        "failed to connect to server: {error} (retrying in {backoff:?} seconds)"
                    );
                    ConnectionState::Disconnected(backoff)
                }
            };
        }
    }

    /// Returns the next message from the server.
    pub async fn next_message(&mut self, state: &mut TuiState) -> Event {
        loop {
            match &mut self.state {
                ConnectionState::Connected { update_stream, .. } => {
                    match update_stream.next().await {
                        Some(Ok(update)) => return update,
                        Some(Err(status)) => {
                            tracing::warn!(
                                "Failed to receive update from server: {status:?}. Retrying in \
                                 {BACKOFF:?} seconds"
                            );
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

    /// Renders the connection state.
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
            Span::raw("Crankshaft server: "),
            Span::raw(self.target.to_string()),
            Span::raw(" "),
            state,
        ])
    }
}
