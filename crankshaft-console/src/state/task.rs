use std::collections::HashMap;

use crankshaft_monitor::proto::Event;
use crankshaft_monitor::proto::EventType;
use crankshaft_monitor::proto::event::Payload::Message;

/// TasksState
#[derive(Debug, Default)]
pub struct TuiTasksState {
    /// tasksmap
    tasks: HashMap<String, Task>,
}

#[derive(Debug)]
pub(crate) struct Tasklogs {
    /// timestamp
    pub timestamp: i64,
    /// message
    pub message: String,
}
/// TasksState
#[derive(Debug)]
pub(crate) struct Task {
    /// id
    id: String,
    /// progress
    progress: i32,
    /// message
    logs: Vec<Tasklogs>,
}

impl TuiTasksState {
    /// Updates the state with a new event
    pub fn update(&mut self, message: Event) {
        let task = self
            .tasks
            .entry(message.event_id.clone())
            .or_insert_with(|| Task::new(message.event_id.clone(), EventType::TaskQueued as i32));
        task.progress = message.event_type;
        if let Some(Message(msg)) = message.payload {
            task.logs.push(Tasklogs {
                timestamp: message.timestamp,
                message: msg,
            });
        }
    }

    /// Returns a reference to the tasks map.
    pub(crate) fn tasks(&self) -> &HashMap<String, Task> {
        &self.tasks
    }

    pub fn set_initial(&mut self, tasks: HashMap<String, i32>) {
        self.tasks = tasks
            .into_iter()
            .map(|(id, progress)| (id.clone(), Task::new(id, progress)))
            .collect();
    }
}

impl Task {
    /// Creates a new Task instance.
    pub fn new(id: String, progress: i32) -> Self {
        let logs = Vec::new();
        Self { id, progress, logs }
    }

    /// Returns the id of the task.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the event type of the task.
    pub fn progress(&self) -> i32 {
        self.progress
    }

    pub fn logs(&self) -> &[Tasklogs] {
        &self.logs
    }

    pub fn event_type(&self) -> EventType {
        EventType::from_i32(self.progress).unwrap_or(EventType::Unspecified)
    }

    pub fn latest_log(&self) -> Option<&Tasklogs> {
        self.logs.last()
    }
}
