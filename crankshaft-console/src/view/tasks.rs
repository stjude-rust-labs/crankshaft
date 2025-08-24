//! Renders the tasks view.
use ratatui::Frame;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Cell;
use ratatui::widgets::Row;
use ratatui::widgets::Table;

use crate::state::task::TuiTasksState;

/// Renders the tasks view.
pub(crate) fn render_tasks(frame: &mut Frame<'_>, tasks_state: &TuiTasksState) {
    let area = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Min(0)])
        .split(frame.area());

    let headers = [
        "Task ID",
        "Name",
        "TES ID",
        "Status",
        "Last Update",
        "Message",
    ]
    .iter()
    .map(|h| {
        Cell::from(*h).style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    });
    let header = Row::new(headers).height(1).bottom_margin(1);

    let rows = tasks_state.tasks().values().map(|task| {
        Row::new([
            Cell::from(task.id().to_string()),
            Cell::from(task.name()),
            Cell::from(task.tes_id().unwrap_or_default()),
            Cell::from(task.status()),
            Cell::from(task.timestamp().to_string()),
            Cell::from(task.message()),
        ])
    });

    let table = Table::new(
        rows,
        [
            Constraint::Max(10),
            Constraint::Max(20),
            Constraint::Max(20),
            Constraint::Max(10),
            Constraint::Max(30),
            Constraint::Fill(1),
        ],
    )
    .header(header)
    .block(Block::default().title("Tasks").borders(Borders::ALL))
    .row_highlight_style(Style::default().bg(Color::DarkGray));

    frame.render_widget(table, area[0]);
}
