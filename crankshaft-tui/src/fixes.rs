use ratatui::style::Style;
use ratatui::text::Span;

pub trait StylizeSpan {
    fn with_style(self, style: Style) -> Span<'static>;
}

impl StylizeSpan for String {
    fn with_style(self, style: Style) -> Span<'static> {
        Span::styled(self, style)
    }
}

impl StylizeSpan for &str {
    fn with_style(self, style: Style) -> Span<'static> {
        Span::styled(self.to_string(), style)
    }
}

// For table cells
pub fn style_to_string(text: &str, style: Style) -> String {
    // In a real implementation, we'd attach styling to the string
    // But for now, we'll just return the string as is, since the table will apply styling
    text.to_string()
}
