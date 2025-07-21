use std::collections::HashMap;

use crankshaft_monitor::proto::Event;
use crankshaft_monitor::proto::EventType;
use crankshaft_monitor::proto::event::Payload::Message;

/// so this type is not named accurately but this will be the task-state that is the
/// proto version
type TaskProgress = i32;

/// TasksState
#[derive(Debug, Default)]
pub struct TasksState {
    /// tasksmap
    tasks: HashMap<String, TaskProgress>,
}

/// TasksState
#[derive(Debug)]
pub(crate) struct Task {
    /// id
    id: String,
    /// event_type
    event_type: EventType,
    /// timestamp
    timestamp: i64,
    /// message
    message: String,
}

impl TasksState {
    /// Updates the state with a new event
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

    /// Returns a reference to the tasks map.
    pub(crate) fn tasks(&self) -> &HashMap<String, Task> {
        &self.tasks
    }

    pub set_initial(&mut self, tasks: HashMap<String, i32>) {
        self.tasks = tasks;
    }
}

impl Task {
    /// Creates a new Task instance.
    pub fn new(id: String) -> Self {
        Self {
            id,
            event_type: EventType::Unspecified,
            timestamp: 0,
            message: "".to_string(),
        }
    }

    /// Returns the id of the task.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the event type of the task.
    pub fn event_type(&self) -> &EventType {
        &self.event_type
    }

    /// Returns the timestamp of the task.
    pub fn timestamp(&self) -> i64 {
        self.timestamp
    }

    /// Returns the message of the task.
    pub fn message(&self) -> &str {
        &self.message
    }
}
