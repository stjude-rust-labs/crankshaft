//! The task module contains the state of the tasks.
use std::collections::BTreeMap;
use std::collections::HashMap;

use crankshaft_monitor::proto::Event;
use crankshaft_monitor::proto::TaskCanceledEvent;
use crankshaft_monitor::proto::TaskCompletedEvent;
use crankshaft_monitor::proto::TaskContainerCreatedEvent;
use crankshaft_monitor::proto::TaskContainerExitedEvent;
use crankshaft_monitor::proto::TaskCreatedEvent;
use crankshaft_monitor::proto::TaskEvents;
use crankshaft_monitor::proto::TaskFailedEvent;
use crankshaft_monitor::proto::TaskPreemptedEvent;
use crankshaft_monitor::proto::TaskStartedEvent;
use crankshaft_monitor::proto::TaskStderrEvent;
use crankshaft_monitor::proto::TaskStdoutEvent;
use crankshaft_monitor::proto::event::EventKind;
use crankshaft_monitor::proto::exit_status::ExitStatusKind;
use prost_types::Timestamp;

/// The `TuiTasksState` struct holds the state of the tasks.
#[derive(Debug, Default)]
pub struct TuiTasksState {
    /// A map of currently executing tasks.
    tasks: BTreeMap<u64, Task>,
}

impl TuiTasksState {
    /// Updates the state with a new event.
    pub fn update(&mut self, event: Event) {
        let id = match &event.event_kind {
            Some(EventKind::Created(TaskCreatedEvent { id, .. }))
            | Some(EventKind::Started(TaskStartedEvent { id, .. }))
            | Some(EventKind::ContainerCreated(TaskContainerCreatedEvent { id, .. }))
            | Some(EventKind::ContainerExited(TaskContainerExitedEvent { id, .. }))
            | Some(EventKind::Stdout(TaskStdoutEvent { id, .. }))
            | Some(EventKind::Stderr(TaskStderrEvent { id, .. }))
            | Some(EventKind::Completed(TaskCompletedEvent { id, .. }))
            | Some(EventKind::Failed(TaskFailedEvent { id, .. }))
            | Some(EventKind::Canceled(TaskCanceledEvent { id, .. }))
            | Some(EventKind::Preempted(TaskPreemptedEvent { id, .. })) => *id,
            None => return,
        };

        if let Some(task) = self.tasks.get_mut(&id) {
            task.events.push(event);
        } else {
            // Insert the task if this is a creation event
            // Otherwise, we missed the creation event, ignore the task.
            if let Some(task) = Task::new(event) {
                self.tasks.insert(id, task);
            }
        }
    }

    /// Returns a reference to the tasks map.
    pub(crate) fn tasks(&self) -> &BTreeMap<u64, Task> {
        &self.tasks
    }

    /// Sets the initial state of the tasks.
    pub fn set_initial(&mut self, tasks: HashMap<u64, TaskEvents>) {
        for (id, events) in tasks {
            if let Some(task) = Task::from_events(events) {
                self.tasks.insert(id, task);
            }
        }
    }
}

/// Represents state for a task.
#[derive(Debug)]
pub struct Task {
    /// The task identifier.
    id: u64,
    /// The name of the task.
    name: String,
    /// The TES id of the task.
    ///
    /// This is `Some` only for tasks from the TES backend.
    tes_id: Option<String>,
    /// The events of the task.
    events: Vec<Event>,
}

impl Task {
    /// Constructs a new task from a creation event.
    ///
    /// Returns `None` if the provided event is not a task creation event.
    fn new(event: Event) -> Option<Self> {
        let (id, name, tes_id) = match &event.event_kind {
            Some(EventKind::Created(event)) => (event.id, event.name.clone(), event.tes_id.clone()),
            _ => return None,
        };

        Some(Self {
            id,
            name,
            tes_id,
            events: vec![event],
        })
    }

    /// Constructs a new task from a complete sent of events.
    ///
    /// Returns `None` if the provided events do not start with a task creation
    /// event.
    fn from_events(events: TaskEvents) -> Option<Self> {
        let (id, name, tes_id) = match &events.events.first()?.event_kind {
            Some(EventKind::Created(event)) => (event.id, event.name.clone(), event.tes_id.clone()),
            _ => return None,
        };

        Some(Self {
            id,
            name,
            tes_id,
            events: events.events,
        })
    }

    /// Gets the id of the task.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Gets the name of the task.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Gets the TES id of the task.
    ///
    /// This is `Some` only for the TES backend.
    pub fn tes_id(&self) -> Option<&str> {
        self.tes_id.as_deref()
    }

    /// Gets the display status of the task.
    pub fn status(&self) -> &str {
        match &self.last_event().event_kind {
            Some(EventKind::Created(_)) => "Created",
            Some(EventKind::Started(_))
            | Some(EventKind::ContainerCreated(_))
            | Some(EventKind::ContainerExited(_))
            | Some(EventKind::Stdout(_))
            | Some(EventKind::Stderr(_)) => "Running",
            Some(EventKind::Completed(_)) => "Completed",
            Some(EventKind::Failed(_)) => "Failed",
            Some(EventKind::Canceled(_)) => "Canceled",
            Some(EventKind::Preempted(_)) => "Preempted",
            None => "Unknown",
        }
    }

    /// Gets the timestamp of the last event for the task.
    pub fn timestamp(&self) -> Timestamp {
        self.last_event()
            .timestamp
            .expect("event should have timestamp")
    }

    /// Gets the message representation of the task's most recent event.
    pub fn message(&self) -> String {
        match &self.last_event().event_kind {
            Some(EventKind::Created(_)) => {
                format!("task `{name}` has been created", name = self.name)
            }
            Some(EventKind::Started(_))
            | Some(EventKind::Stdout(_))
            | Some(EventKind::Stderr(_)) => format!("task `{name}` is running", name = self.name),
            Some(EventKind::ContainerCreated(event)) => format!(
                "created container `{container}` for task `{name}`",
                container = event.container,
                name = self.name
            ),
            Some(EventKind::ContainerExited(event)) => event
                .exit_status
                .and_then(|s| {
                    s.exit_status_kind.map(|k| match k {
                        ExitStatusKind::Code(code) => format!(
                            "container `{container}` has exited with code `{code}` for task \
                             `{name}`",
                            container = event.container,
                            name = self.name
                        ),
                        ExitStatusKind::Signal(signal) => format!(
                            "container `{container}` has exited with signal `{signal}` for task \
                             `{name}`",
                            container = event.container,
                            name = self.name
                        ),
                    })
                })
                .unwrap_or_default(),
            Some(EventKind::Completed(_)) => {
                format!("task `{name}` has completed", name = self.name)
            }
            Some(EventKind::Failed(event)) => format!(
                "task `{name}` has failed: {message}",
                name = self.name,
                message = event.message
            ),
            Some(EventKind::Canceled(_)) => {
                format!("task `{name}` has been canceled", name = self.name)
            }
            Some(EventKind::Preempted(_)) => {
                format!("task `{name}` has been preempted", name = self.name)
            }
            None => Default::default(),
        }
    }

    /// Gets the last event of the task.
    ///
    /// # Panics
    ///
    /// Panics if there are no events for the task.
    fn last_event(&self) -> &Event {
        self.events
            .last()
            .expect("there should always be at least one event")
    }
}
