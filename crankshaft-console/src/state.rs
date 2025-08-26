//! The state module contains the state of the TUI.

use crankshaft_monitor::proto::Event;
use crankshaft_monitor::proto::ServiceStateResponse;

use crate::state::task::TuiTasksState;
use crate::view::View;
use crate::view::styles::Styles;

pub mod task;

/// The `State` struct holds the state of the TUI.
#[derive(Debug)]
pub struct State {
    /// The state of the tasks.
    pub tasks_state: TuiTasksState,
    /// The current view of the TUI.
    pub current_view: View,
    /// Whether the log view is active.
    pub log_view: bool,
}

impl Default for State {
    /// Returns the default state.
    fn default() -> Self {
        Self {
            tasks_state: TuiTasksState::default(),
            current_view: View::default(),
            log_view: false,
        }
    }
}

impl State {
    /// Sets the initial state of the TUI.
    pub fn set_initial_state(&mut self, state: ServiceStateResponse) {
        self.tasks_state.set_initial(state.tasks);
    }

    /// Updates the state of the TUI.
    pub fn update(&mut self, _styles: &Styles, _view: View, event: Event) {
        self.tasks_state.update(event);
    }

    /// Returns the task state.
    pub fn task_state(&self) -> &TuiTasksState {
        &self.tasks_state
    }

    /// Returns a mutable reference to the task state.
    pub fn task_state_mut(&mut self) -> &mut TuiTasksState {
        &mut self.tasks_state
    }
}
