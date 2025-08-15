//! The task module contains the state of the tasks.
use std::collections::HashMap;

use crankshaft_monitor::proto::Event;
use crankshaft_monitor::proto::EventType;
use crankshaft_monitor::proto::event::Payload::Message;

/// The `TuiTasksState` struct holds the state of the tasks.
#[derive(Debug, Default)]
pub struct TuiTasksState {
    /// A map of tasks.
    tasks: HashMap<String, Task>,
}

/// The `Tasklogs` struct holds the logs of a task.
#[derive(Debug)]
pub(crate) struct Tasklogs {
    /// The timestamp of the log.
    pub timestamp: i64,
    /// The message of the log.
    pub message: String,
}

/// The `Task` struct holds the state of a task.
#[derive(Debug)]
pub(crate) struct Task {
    /// The id of the task.
    id: String,
    /// The progress of the task.
    progress: i32,
    /// The logs of the task.
    logs: Vec<Tasklogs>,
}

impl TuiTasksState {
    /// Updates the state with a new event.
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

    /// Sets the initial state of the tasks.
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
    pub fn event_type(&self) -> EventType {
        EventType::try_from(self.progress).unwrap_or(EventType::Unspecified)
    }

    /// Returns the latest log of the task.
    pub fn latest_log(&self) -> Option<&Tasklogs> {
        self.logs.last()
    }
}
