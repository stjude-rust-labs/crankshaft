//! Crate for monitoring crankshaft events.
use std::net::SocketAddr;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use anyhow::Result;
use proto::Event;
use proto::monitor_server::MonitorServer;
use server::MonitorService;
use tokio::sync::broadcast;

pub mod proto;
pub mod server;

/// This represents the default capacity of the broadcast channel used for event
/// communication.
const DEFAULT_CHANNEL_CAPACITY: usize = 16;

/// JoinHandle type alias for the tokio task handle.
pub type JoinHandle = tokio::task::JoinHandle<Result<(), tonic::transport::Error>>;

/// The main external API to start the Crankshaft monitor.
pub fn start_monitoring(addr: SocketAddr) -> Result<(broadcast::Sender<Event>, JoinHandle)> {
    let (event_sender, event_receiver) = broadcast::channel(DEFAULT_CHANNEL_CAPACITY);
    let monitor_service = MonitorService::new(event_receiver);
    let server = MonitorServer::new(monitor_service);

    let server_handle = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(server)
            .serve(addr)
            .await
    });
    Ok((event_sender, server_handle))
}

/// current timestamp as i64
pub fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System time before UNIX epoch")
        .as_millis() as i64
}

/// Sends an event through a broadcast channel.
///
/// No event is sent if the specified broadcast channel is `None`.

#[macro_export]
macro_rules! send_event {
    // Message literal
    ($sender:expr, $task_id:expr, $event_type:expr, $message:literal) => {
        if let Some(sender) = $sender.as_ref() {
            let message = format!($message);
            let _ = sender.send($crate::proto::Event {
                task_id: $task_id.to_owned(),
                event_type: $event_type as i32,
                timestamp: $crate::now_millis(),
                payload: Some($crate::proto::event::Payload::Message(message)),
            });
        }
    };

    // Message from format!
    ($sender:expr, $task_id:expr, $event_type:expr, $fmt:literal, $($arg:tt)*) => {
        if let Some(sender) = $sender.as_ref() {
            let message = format!($fmt, $($arg)*);
            let _ = sender.send($crate::proto::Event {
                task_id: $task_id.to_owned(),
                event_type: $event_type as i32,
                timestamp: $crate::now_millis(),
                payload: Some($crate::proto::event::Payload::Message(message)),
            });
        }
    };

    // ContainerResources payload
    ($sender:expr, $task_id:expr, $event_type:expr, container_resource = $res:expr) => {
        if let Some(sender) = $sender.as_ref() {
            let _ = sender.send($crate::proto::Event {
                task_id: $task_id.to_owned(),
                event_type: $event_type as i32,
                timestamp: $crate::now_millis(),
                payload: Some($crate::proto::event::Payload::ContainerResources($res)),
            });
        }
    };

    // ServiceResources payload
    ($sender:expr, $task_id:expr, $event_type:expr, service_resource = $res:expr) => {
        if let Some(sender) = $sender.as_ref() {
            let _ = sender.send($crate::proto::Event {
                task_id: $task_id.to_owned(),
                event_type: $event_type as i32,
                timestamp: $crate::now_millis(),
                payload: Some($crate::proto::event::Payload::ServiceResources($res)),
            });
        }
    };

    ($sender:expr, $task_id:expr, $event_type:expr, $($arg:tt)*) => {
        if let Some(sender) = $sender.as_ref() {
            let message = format!("{}",$($arg)*);
            let _ = sender.send($crate::proto::Event {
                task_id: $task_id.to_owned(),
                event_type: $event_type as i32,
                timestamp: $crate::now_millis(),
                payload: Some($crate::proto::event::Payload::Message(message)),
            });
        }
    };
}
