//! Provides styling for the TUI.
use ratatui::style::Color;
use ratatui::style::Style;

/// Represents the styles for the TUI.
#[derive(Debug, Clone, Default)]
pub struct Styles;

impl Styles {
    /// Creates a new `Styles` instance.
    pub fn new() -> Self {
        Styles
    }

    /// Returns a style with the given foreground color.
    pub fn fg(&self, color: Color) -> Style {
        Style::default().fg(color)
    }
}
