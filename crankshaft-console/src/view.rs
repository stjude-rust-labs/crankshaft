//! The view module contains the rendering logic for the TUI.
use std::borrow::Cow;

use ratatui::Frame;
use ratatui::style::Style;
use ratatui::text::Span;

use crate::state::State;

pub mod styles;
pub mod tasks;

/// The `View` enum represents the different views of the TUI.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub enum View {
    /// The tasks view.
    #[default]
    Tasks,
}

impl View {
    /// Renders the view.
    pub fn render(self, frame: &mut Frame<'_>, state: &mut State) {
        match self {
            View::Tasks => tasks::render_tasks(frame, state.task_state_mut()),
        }
    }
}

/// Returns a bolded span.
pub(crate) fn bold<'a>(text: impl Into<Cow<'a, str>>) -> Span<'a> {
    Span::styled(text, Style::default())
}
