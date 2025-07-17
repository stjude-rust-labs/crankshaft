use std::error::Error;
use std::time::Duration;

use crankshaft_monitor::proto::Event;
use crankshaft_monitor::proto::SubscribeEventsRequest;
use crankshaft_monitor::proto::monitor_client::MonitorClient;
use futures_util::StreamExt;
use tokio::net::UnixStream;
use tonic::Streaming;
use tonic::transport::Channel;
use tonic::transport::Endpoint;
use tonic::transport::Uri;

#[derive(Debug)]
pub struct Connection {
    target: Uri,
    state: State,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
enum State {
    Connected {
        client: MonitorClient<Channel>,
        update_stream: Box<Streaming<Event>>,
    },
    Disconnected(Duration),
}

impl Connection {
    const BACKOFF: Duration = Duration::from_millis(500);

    pub fn new(target: Uri) -> Self {
        Self {
            target,
            state: State::Disconnected(Duration::from_secs(0)),
        }
    }

    async fn connect(&mut self) {
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
                let update_stream =
                    Box::new(client.subscribe_events(update_request).await?.into_inner());
                Ok::<State, Box<dyn Error + Send + Sync>>(State::Connected {
                    client,
                    update_stream,
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

    pub fn render(&self, styles: &crate::view::styles::Styles) -> ratatui::text::Line {
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
                format!("(RECONNECTING IN {:?})", d),
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
