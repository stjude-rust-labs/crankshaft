use std::collections::HashMap;

use crankshaft_monitor::proto::Event;
use crankshaft_monitor::proto::EventType;
use crankshaft_monitor::proto::event::Payload::Message;

#[derive(Debug, Default)]
pub struct TasksState {
    tasks: HashMap<String, Task>,
}

#[derive(Debug)]
pub(crate) struct Task {
    id: String,
    event_type: EventType,
    timestamp: i64,
    message: String,
}

impl TasksState {
    pub fn update(&mut self, message: Event) {
        let task = self
            .tasks
            .entry(message.event_id.clone())
            .or_insert_with(|| Task::new(message.event_id.clone()));
        task.event_type = EventType::try_from(message.event_type).unwrap_or(EventType::Unspecified);
        task.timestamp = message.timestamp;
        if let Some(Message(msg)) = message.payload {
            task.message = msg;
        }
    }

    pub(crate) fn tasks(&self) -> &HashMap<String, Task> {
        &self.tasks
    }
}

impl Task {
    pub fn new(id: String) -> Self {
        Self {
            id,
            event_type: EventType::Unspecified,
            timestamp: 0,
            message: "".to_string(),
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn event_type(&self) -> &EventType {
        &self.event_type
    }

    pub fn timestamp(&self) -> i64 {
        self.timestamp
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}
