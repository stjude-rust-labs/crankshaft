//! The view module contains the rendering logic for the TUI.
use std::borrow::Cow;

use ratatui::Frame;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Clear;
use ratatui::widgets::Paragraph;

use crate::state::State;

pub mod styles;
pub mod tasks;

/// The `View` enum represents the different views of the TUI.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub enum View {
    /// The tasks view.
    #[default]
    Tasks,
    /// Task cancel popup window
    Cancel,
}

impl View {
    /// Renders the view.
    pub fn render(self, frame: &mut Frame<'_>, state: &mut State) {
        tasks::render_tasks(frame, state.task_state_mut());
        match self {
            View::Tasks => (),
            View::Cancel => render_cancel_popup(frame),
        }
    }
}

/// Renders the cancel popup.
fn render_cancel_popup(frame: &mut Frame<'_>) {
    let text = vec![
        Line::from("Are you sure you want to cancel this task?"),
        Line::from("(y/n)"),
    ];
    let paragraph =
        Paragraph::new(text).block(Block::default().title("Cancel Task").borders(Borders::ALL));
    let area = centered_rect(50, 20, frame.area());
    frame.render_widget(Clear, area);
    frame.render_widget(paragraph, area);
}

/// Returns a centered rect.
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}

/// Returns a bolded span.
pub(crate) fn bold<'a>(text: impl Into<Cow<'a, str>>) -> Span<'a> {
    Span::styled(text, Style::default())
}
