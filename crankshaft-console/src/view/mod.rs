pub mod styles;
pub mod tasks;

use std::borrow::Cow;

use ratatui::Frame;
use ratatui::style::Style;
use ratatui::text::Span;

use crate::state::State;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum View {
    Tasks,
}

impl View {
    pub fn render(self, f: &mut Frame<'_>, state: &mut State) {
        match self {
            View::Tasks => tasks::render_tasks(f, state.tasks_state()),
        }
    }
}
pub(crate) fn bold<'a>(text: impl Into<Cow<'a, str>>) -> Span<'a> {
    Span::styled(text, Style::default())
}
