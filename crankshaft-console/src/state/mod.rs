//! The state module contains the state of the TUI.
use std::collections::HashMap;

use crankshaft_monitor::proto::Event;
use crankshaft_monitor::proto::EventType;
use crankshaft_monitor::proto::Resources;
use resource::ResourceState;

use crate::state::task::TuiTasksState;
use crate::view::View;
use crate::view::styles::Styles;

/// The resource state module.
pub mod resource;
/// The task state module.
pub mod task;

/// The `State` struct holds the state of the TUI.
#[derive(Debug)]
pub struct State {
    /// The state of the tasks.
    pub tasks_state: TuiTasksState,
    /// The state of the resources.
    pub resource_state: ResourceState,
    /// The current view of the TUI.
    pub current_view: View,
}

impl Default for State {
    /// Returns the default state.
    fn default() -> Self {
        Self {
            tasks_state: TuiTasksState::default(),
            resource_state: ResourceState::default(),
            current_view: View::default(),
        }
    }
}

impl State {
    /// Sets the initial state of the TUI.
    pub fn set_initial_state(&mut self, tasks: HashMap<String, i32>, resources: Option<Resources>) {
        self.tasks_state.set_initial(tasks);
        self.resource_state.set_initial(resources);
    }

    /// Updates the state of the TUI.
    pub fn update(&mut self, _styles: &Styles, _view: View, message: Event) {
        match EventType::try_from(message.event_type) {
            Ok(EventType::ServiceStarted) | Ok(EventType::ContainerStarted) => {
                self.resource_state.update(message)
            }
            _ => self.tasks_state.update(message),
        }
    }

    /// Returns the task state.
    pub fn task_state(&self) -> &TuiTasksState {
        &self.tasks_state
    }

    /// Returns the resource state.
    pub fn resource_state(&self) -> &ResourceState {
        &self.resource_state
    }
}
