use ratatui::style::Color;
use ratatui::style::Style;

#[derive(Debug, Clone, Default)]
pub struct Styles;

impl Styles {
    pub fn new() -> Self {
        Styles
    }

    pub fn fg(&self, color: Color) -> Style {
        Style::default().fg(color)
    }
}
