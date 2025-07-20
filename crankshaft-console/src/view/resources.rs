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

    let rows = resource_state.resources().values().map(|r| {
        let row = vec![
            r.id.clone(),
            r.cpu.map_or("—".into(), |v| format!("{:.2}", v)),
            r.max_cpu.map_or("—".into(), |v| format!("{:.2}", v)),
            r.memory.map_or("—".into(), |v| format!("{:.2}", v)),
            r.max_memory.map_or("—".into(), |v| format!("{:.2}", v)),
            r.nodes.map_or("—".into(), |v| format!("{:.0}", v)),
        ];
        Row::new(row.into_iter().map(Cell::from))
    });

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
