use crate::state::TasksState;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

pub(crate) fn render_tasks(frame: &mut Frame<'_>, tasks_state: &TasksState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Percentage(100)].as_ref())
        .split(frame.size());

    let header_cells = ["Task ID", "Event Type", "Timestamp", "Message"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Red)));
    let header = Row::new(header_cells)
        .style(Style::default())
        .height(1)
        .bottom_margin(1);

    let rows = tasks_state.tasks().values().map(|task| {
        let items = vec![
            task.id().to_string(),
            format!("{:?}", task.event_type()),
            task.timestamp().to_string(),
            task.message().to_string(),
        ];
        Row::new(items.into_iter().map(Cell::from))
    });

    let table = Table::new(
        rows,
        &[
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title("Tasks"));

    frame.render_widget(table, chunks[0]);
}