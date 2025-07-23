//! States defined in the tui goes here
use std::collections::HashMap;

use crankshaft_monitor::proto::Event;
use crankshaft_monitor::proto::EventType;
use crankshaft_monitor::proto::Resources;
use resource::ResourceState;

use crate::state::task::TuiTasksState;
use crate::view::View;
use crate::view::styles::Styles;
pub mod resource;
pub mod task;

#[derive(Debug)]
pub struct State {
    pub tasks_state: TuiTasksState,
    pub resource_state: ResourceState,
    pub current_view: View,
}

impl Default for State {
    fn default() -> Self {
        Self {
            tasks_state: TuiTasksState::default(),
            resource_state: ResourceState::default(),
            current_view: View::default(),
        }
    }
}

impl State {
    pub fn set_initial_state(&mut self, tasks: HashMap<String, i32>, resources: Option<Resources>) {
        self.tasks_state.set_initial(tasks);
        self.resource_state.set_initial(resources);
    }

    pub fn update(&mut self, _styles: &Styles, _view: View, message: Event) {
        match EventType::try_from(message.event_type) {
            Ok(EventType::ServiceStarted) | Ok(EventType::ContainerStarted) => {
                self.resource_state.update(message)
            }
            _ => self.tasks_state.update(message),
        }
    }

    pub fn task_state(&self) -> &TuiTasksState {
        &self.tasks_state
    }

    pub fn resource_state(&self) -> &ResourceState {
        &self.resource_state
    }
}
