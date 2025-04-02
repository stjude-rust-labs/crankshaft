use ratatui::style::{Color, Modifier, Style};

pub struct Theme {
    pub app_title_style: Style,
    pub title_style: Style,
    pub header_style: Style,
    pub text_style: Style,
    pub selected_style: Style,
    pub highlight_style: Style,
    pub button_style: Style,
    pub button_hover_style: Style,
    pub block_style: Style,
    pub error_style: Style,
    pub warning_style: Style,
    pub success_style: Style,
    pub info_style: Style,
    pub status_colors: StatusColors,
}

pub struct StatusColors {
    pub running: Color,
    pub stopped: Color,
    pub error: Color,
    pub inactive: Color,
    pub active: Color,
}

impl Default for Theme {
    fn default() -> Self {
        let status_colors = StatusColors {
            running: Color::Green,
            stopped: Color::Red,
            error: Color::LightRed,
            inactive: Color::DarkGray,
            active: Color::LightGreen,
        };

        Self {
            app_title_style: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            title_style: Style::default()
                .fg(Color::White)
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
            header_style: Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            text_style: Style::default().fg(Color::White),
            selected_style: Style::default().bg(Color::Blue),
            highlight_style: Style::default()
                .fg(Color::Black)
                .bg(Color::LightCyan),
            button_style: Style::default()
                .fg(Color::White)
                .bg(Color::DarkGray),
            button_hover_style: Style::default()
                .fg(Color::White)
                .bg(Color::Blue),
            block_style: Style::default().fg(Color::Gray),
            error_style: Style::default().fg(Color::Red),
            warning_style: Style::default().fg(Color::Yellow),
            success_style: Style::default().fg(Color::Green),
            info_style: Style::default().fg(Color::Blue),
            status_colors,
        }
    }
}
