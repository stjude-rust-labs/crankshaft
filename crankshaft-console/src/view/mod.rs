use std::borrow::Cow;

use ratatui::Frame;
use ratatui::style::Style;
use ratatui::text::Span;

use crate::state::State;

pub mod resources;
pub mod styles;
pub mod tasks;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum View {
    Tasks,
    Resources,
}

impl View {
    pub fn render(self, frame: &mut Frame<'_>, state: &mut State) {
        match self {
            View::Tasks => tasks::render_tasks(frame, state.task_state()),
            View::Resources => resources::render_resource(frame, state.resource_state()),
        }
    }
}

pub(crate) fn bold<'a>(text: impl Into<Cow<'a, str>>) -> Span<'a> {
    Span::styled(text, Style::default())
}

impl Default for View {
    fn default() -> Self {
        View::Tasks
    }
}
