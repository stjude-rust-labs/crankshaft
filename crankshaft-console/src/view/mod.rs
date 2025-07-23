//! The view module contains the rendering logic for the TUI.
use std::borrow::Cow;

use ratatui::Frame;
use ratatui::style::Style;
use ratatui::text::Span;

use crate::state::State;

/// The resources module contains the rendering logic for the resources view.
pub mod resources;

/// The styles module contains the styling for the TUI.
pub mod styles;

/// The tasks module contains the rendering logic for the tasks view.
pub mod tasks;

/// The `View` enum represents the different views of the TUI.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub enum View {
    /// The tasks view.
    #[default]
    Tasks,
    /// The resources view.
    Resources,
}

impl View {
    /// Renders the view.
    pub fn render(self, frame: &mut Frame<'_>, state: &mut State) {
        match self {
            View::Tasks => tasks::render_tasks(frame, state.task_state()),
            View::Resources => resources::render_resource(frame, state.resource_state()),
        }
    }
}

/// Returns a bolded span.
pub(crate) fn bold<'a>(text: impl Into<Cow<'a, str>>) -> Span<'a> {
    Span::styled(text, Style::default())
}
