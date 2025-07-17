use std::collections::HashMap;

use crankshaft_engine::task::Resources;
use crankshaft_monitor::proto::Event;
use crankshaft_monitor::proto::EventType;
use task::TasksState;

use crate::state::resource::ResourceState;
use crate::view::View;
use crate::view::styles::Styles;

mod resource;
mod task;

#[derive(Debug, Default)]
pub struct State {
    tasks_state: TasksState,
    resource_state: ResourceState,
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
}
