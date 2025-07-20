//! States defined in the tui goes here
use crankshaft_monitor::proto::Event;
use crankshaft_monitor::proto::EventType;
use resource::ResourceState;
use task::TasksState;

use crate::view::View;
use crate::view::styles::Styles;
pub mod resource;
pub mod task;

#[derive(Debug, Default)]
pub struct State {
    tasks_state: TasksState,
    resource_state: ResourceState,
    pub current_view: View,
}

impl State {
    pub fn update(&mut self, _styles: &Styles, _view: View, message: Event) {
        match EventType::try_from(message.event_type) {
            Ok(EventType::ServiceStarted) | Ok(EventType::ContainerStarted) => {
                self.resource_state.update(message)
            }
            _ => self.tasks_state.update(message),
        }
    }

    pub fn task_state(&self) -> &TasksState {
        &self.tasks_state
    }

    pub fn resource_state(&self) -> &ResourceState {
        &self.resource_state
    }
}
