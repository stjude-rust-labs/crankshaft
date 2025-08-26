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
use ratatui::widgets::TableState;

use crate::state::task::TuiTasksState;

/// Renders the tasks view.
pub(crate) fn render_tasks(frame: &mut Frame<'_>, tasks_state: &mut TuiTasksState) {
    let area = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Min(0)])
        .split(frame.area());

    let headers = [
        "",
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

    let rows = tasks_state.tasks().values().enumerate().map(|(i, task)| {
        let mut row = vec![
            Cell::from(task.id().to_string()),
            Cell::from(task.name()),
            Cell::from(task.tes_id().unwrap_or_default()),
            Cell::from(task.status()),
            Cell::from(task.timestamp().to_string()),
            Cell::from(task.message()),
        ];
        if tasks_state.selected_task_index == Some(i) {
            row.insert(0, Cell::from("→"));
        } else {
            row.insert(0, Cell::from(""));
        }
        Row::new(row)
    });

    let mut table_state = TableState::default();
    table_state.select(tasks_state.selected_task_index);

    let table = Table::new(
        rows,
        [
            Constraint::Max(1),
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

    frame.render_stateful_widget(table, area[0], &mut table_state);
}
