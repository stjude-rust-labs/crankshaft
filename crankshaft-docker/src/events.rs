//! Event utils
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use crankshaft_monitor::proto::Event;
use crankshaft_monitor::proto::EventType;
use tokio::sync::broadcast;

/// Sends a Event through a braodcast channel
pub fn send_event(
    sender: &Option<broadcast::Sender<Event>>,
    task_id: &String,
    event_type: EventType,
    message: impl Into<String>,
) {
    if let Some(sender) = sender {
        let _ = sender.send(Event {
            task_id: task_id.to_owned(),
            event_type: event_type as i32,
            timestamp: now_millis(),
            message: message.into(),
        });
    }
}

/// current timestamp as i64
fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System time before UNIX epoch")
        .as_millis() as i64
}
