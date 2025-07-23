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

use crate::state::resource::ResourceState;

pub(crate) fn render_resource(frame: &mut Frame<'_>, resource_state: &ResourceState) {
    let area = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Min(0)])
        .split(frame.size());

    let headers = ["Task ID", "CPU", "Max CPU", "Memory", "Max Memory", "Nodes"]
        .iter()
        .map(|h| {
            Cell::from(*h).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        });
    let header = Row::new(headers).height(1).bottom_margin(1);

    let r = resource_state.resources();
    let row = vec![
        r.cpu.to_string(),
        r.max_cpu.to_string(),
        r.memory.to_string(),
        r.max_memory.to_string(),
        r.nodes.to_string(),
    ];
    let rows = vec![Row::new(row.into_iter().map(Cell::from)).height(1)];

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(20),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
            Constraint::Percentage(20),
        ],
    )
    .header(header)
    .block(Block::default().title("Resources").borders(Borders::ALL))
    .highlight_style(Style::default().bg(Color::DarkGray));

    frame.render_widget(table, area[0]);
}
