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

pub(crate) fn render_tasks(frame: &mut Frame<'_>, tasks_state: &TuiTasksState) {
    let area = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Min(0)])
        .split(frame.size());

    let headers = ["Task ID", "Event Type", "Timestamp", "Message"]
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
        let (timestamp, message) = task
            .latest_log()
            .map(|log| (log.timestamp.to_string(), log.message.clone()))
            .unwrap_or_else(|| ("-".into(), "-".into()));

        let row = vec![
            task.id().to_string(),
            format!("{:?}", task.event_type()),
            timestamp,
            message,
        ];
        Row::new(row.into_iter().map(Cell::from))
    });

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ],
    )
    .header(header)
    .block(Block::default().title("Tasks").borders(Borders::ALL))
    .highlight_style(Style::default().bg(Color::DarkGray));

    frame.render_widget(table, area[0]);
}
