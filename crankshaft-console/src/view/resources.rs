//! Renders the resources view.
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

/// Renders the resources view.
pub(crate) fn render_resource(frame: &mut Frame<'_>, resource_state: &ResourceState) {
    let area = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Min(0)])
        .split(frame.size());

    let headers = ["CPU", "Max CPU", "Memory", "Max Memory", "Nodes"]
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
    let memory = r.memory / (1024.0 * 1024.0 * 1024.0);
    let max_memory = r.max_memory / (1024.0 * 1024.0 * 1024.0);
    let row = [
        Cell::from(format!("{:.2} cores", r.cpu)),
        Cell::from(format!("{:.2} cores", r.max_cpu)),
        Cell::from(format!("{memory:.2} GB")),
        Cell::from(format!("{max_memory:.2} GB")),
        Cell::from(r.nodes.to_string()),
    ];
    let rows = [Row::new(row).height(1)];

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
