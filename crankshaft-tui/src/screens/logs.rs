use std::collections::HashMap;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};
use serde_json::Value;
use crate::ui::Theme;

const MAX_LOG_LINES: usize = 100;

/// Log viewer screen for monitoring service and container logs.
pub struct LogsScreen {
    pub following: bool,
    pub service_name: String,
    pub log_levels: Vec<&'static str>,
    pub logs: Vec<String>,
    pub follow_button_label: String,
    pub follow_button_variant: String,
    pub refresh_status: String,
    pub notification: Option<String>,
    pub active_filter: Option<String>,
}

impl LogsScreen {
    /// Create a new LogsScreen.
    pub fn new() -> Self {
        Self {
            following: false,
            service_name: "frontend".to_string(),
            log_levels: vec!["INFO", "WARN", "ERROR", "DEBUG"],
            logs: Vec::new(),
            follow_button_label: "Follow".to_string(),
            follow_button_variant: "primary".to_string(),
            refresh_status: "Auto-refresh: Off".to_string(),
            notification: None,
            active_filter: None,
        }
    }

    /// Render the logs screen
    pub fn render(&mut self, f: &mut Frame, _data: &Value, buttons: &mut HashMap<String, Rect>, _clickable_areas: &mut HashMap<String, Rect>, theme: &Theme) {
        let size = f.size();
        
        // Define the main chunks using Layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints(
                [
                    Constraint::Length(3),  // Header
                    Constraint::Min(10),    // Content
                    Constraint::Length(3),  // Footer
                ]
                .as_ref(),
            )
            .split(size);

        // Header
        let header = Paragraph::new("SERVICE LOGS")
            .style(theme.title_style)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(theme.block_style))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(header, chunks[0]);

        // Content area
        let content_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(2), // Title and subtitle
                    Constraint::Length(3), // Filter buttons
                    Constraint::Min(5),    // Logs display
                    Constraint::Length(3), // Logs actions (buttons)
                ]
                .as_ref(),
            )
            .split(chunks[1]);

        // Title and subtitle
        let header_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(content_chunks[0]);
            
        let title = Paragraph::new("SERVICE LOGS")
            .style(theme.text_style.add_modifier(Modifier::BOLD));
        f.render_widget(title, header_layout[0]);
        
        let subtitle_text = format!(
            "{} | {}",
            "Real-time container logs", self.refresh_status
        );
        let subtitle = Paragraph::new(subtitle_text)
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(subtitle, header_layout[1]);

        // Filter buttons
        let filter_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
            ].as_ref())
            .split(content_chunks[1]);
        
        let filters = [
            ("filter_all", "All", Color::White, filter_chunks[0]),
            ("filter_info", "INFO", Color::Green, filter_chunks[1]),
            ("filter_warn", "WARN", Color::Yellow, filter_chunks[2]),
            ("filter_error", "ERROR", Color::Red, filter_chunks[3]),
            ("filter_debug", "DEBUG", Color::Blue, filter_chunks[4]),
        ];
        
        for (id, label, color, rect) in filters {
            let is_active = match &self.active_filter {
                Some(filter) => filter == id,
                None => id == "filter_all",
            };
            
            let style = if is_active {
                Style::default().fg(color).bg(Color::DarkGray).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(color)
            };
            
            let filter_button = Paragraph::new(label)
                .style(style)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded))
                .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(filter_button, rect);
            buttons.insert(id.to_string(), rect);
        }

        // Logs display
        let filtered_logs = if let Some(filter) = &self.active_filter {
            match filter.as_str() {
                "filter_info" => self.logs.iter().filter(|l| l.contains("INFO")).cloned().collect(),
                "filter_warn" => self.logs.iter().filter(|l| l.contains("WARN")).cloned().collect(),
                "filter_error" => self.logs.iter().filter(|l| l.contains("ERROR")).cloned().collect(),
                "filter_debug" => self.logs.iter().filter(|l| l.contains("DEBUG")).cloned().collect(),
                _ => self.logs.clone(),
            }
        } else {
            self.logs.clone()
        };
        
        let logs_text = filtered_logs.iter()
            .map(|log| {
                let log_str = log.clone();
                if log.contains("INFO") {
                    Line::from(Span::styled(log_str, Style::default().fg(Color::Green)))
                } else if log.contains("WARN") {
                    Line::from(Span::styled(log_str, Style::default().fg(Color::Yellow)))
                } else if log.contains("ERROR") {
                    Line::from(Span::styled(log_str, Style::default().fg(Color::Red)))
                } else if log.contains("DEBUG") {
                    Line::from(Span::styled(log_str, Style::default().fg(Color::Blue)))
                } else {
                    Line::from(Span::raw(log_str))
                }
            })
            .collect::<Vec<_>>();

        let logs_paragraph = Paragraph::new(Text::from(logs_text))
            .style(theme.text_style)
            .block(Block::default()
                .title("CONTAINER LOGS")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(theme.block_style))
            .wrap(Wrap { trim: true });
        f.render_widget(logs_paragraph, content_chunks[2]);

        // Action buttons
        let action_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(34),
                Constraint::Percentage(33),
            ].as_ref())
            .split(content_chunks[3]);
        
        // Back button
        let back_button = Paragraph::new("Back")
            .style(theme.button_style)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(back_button, action_chunks[0]);
        buttons.insert("back".to_string(), action_chunks[0]);
        
        // Follow button
        let follow_style = if self.following {
            theme.success_style
        } else {
            theme.button_style
        };
        
        let follow_button = Paragraph::new(self.follow_button_label.clone())
            .style(follow_style)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(follow_button, action_chunks[1]);
        buttons.insert("follow".to_string(), action_chunks[1]);
        
        // Clear button
        let clear_button = Paragraph::new("Clear")
            .style(theme.button_style)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(clear_button, action_chunks[2]);
        buttons.insert("clear".to_string(), action_chunks[2]);

        // Footer
        let footer_text = match &self.notification {
            Some(msg) => format!("NOTIFY: {}", msg),
            None => "ESC: Back | f: Follow | c: Clear | r: Refresh".to_string(),
        };
        
        let footer = Paragraph::new(footer_text)
            .style(theme.text_style)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(theme.block_style))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(footer, chunks[2]);
    }

    /// Handle mounting of the screen.
    pub fn on_mount(&mut self) {
        self.generate_sample_logs();
        self.notification = Some("Log screen loaded".to_string());
    }

    /// Generate sample log entries.
    pub fn generate_sample_logs(&mut self) {
        let sample_logs = vec![
            "[2024-03-09 10:00:01] [INFO] Application starting...".to_string(),
            "[2024-03-09 10:00:02] [INFO] Loading configuration from /etc/config.json".to_string(),
            "[2024-03-09 10:00:03] [DEBUG] Configuration loaded: { \"port\": 3000, \"env\": \"production\" }".to_string(),
            "[2024-03-09 10:00:05] [INFO] Connecting to database at db:5432".to_string(),
            "[2024-03-09 10:00:07] [INFO] Database connection established".to_string(),
            "[2024-03-09 10:00:10] [INFO] Starting web server on port 3000".to_string(),
            "[2024-03-09 10:00:15] [INFO] Server listening at http://0.0.0.0:3000".to_string(),
            "[2024-03-09 10:01:02] [INFO] Received request GET /api/status".to_string(),
            "[2024-03-09 10:01:03] [DEBUG] Request headers: { \"auth\": \"***\" }".to_string(),
            "[2024-03-09 10:01:04] [INFO] Response sent: 200 OK (15ms)".to_string(),
            "[2024-03-09 10:01:10] [WARN] Slow database query detected (325ms)".to_string(),
            "[2024-03-09 10:01:15] [ERROR] Failed to connect to external API: Timeout".to_string(),
        ];
        self.logs = sample_logs;
    }

    /// Add a new log entry if following is enabled.
    pub fn add_new_log_entry(&mut self) {
        if !self.following {
            return;
        }
        
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let level = if self.logs.len() % 4 == 0 { "DEBUG" }
                    else if self.logs.len() % 3 == 0 { "ERROR" }
                    else if self.logs.len() % 2 == 0 { "WARN" }
                    else { "INFO" };
                    
        let messages = match level {
            "INFO" => vec!["Received request", "Processing data", "Response sent", "Connection established"],
            "WARN" => vec!["High CPU usage", "Slow query detected", "Low disk space", "High memory usage"],
            "ERROR" => vec!["Connection refused", "Database timeout", "API error", "Authentication failed"],
            "DEBUG" => vec!["Request parameters", "Cache statistics", "Query execution plan", "Response headers"],
            _ => vec!["Unknown log entry"],
        };
        
        let message = messages[self.logs.len() % messages.len()];
        let log_entry = format!("[{}] [{}] {}", timestamp, level, message);
        
        self.logs.push(log_entry);
        if self.logs.len() > MAX_LOG_LINES {
            self.logs = self.logs[self.logs.len() - MAX_LOG_LINES..].to_vec();
        }
    }

    /// Toggle log following.
    pub fn action_toggle_follow(&mut self) {
        self.following = !self.following;
        if self.following {
            self.follow_button_label = "Following".to_string();
            self.follow_button_variant = "success".to_string();
            self.refresh_status = "Auto-refresh: On".to_string();
            self.notification = Some("Auto-refresh enabled".to_string());
        } else {
            self.follow_button_label = "Follow".to_string();
            self.follow_button_variant = "primary".to_string();
            self.refresh_status = "Auto-refresh: Off".to_string();
            self.notification = Some("Auto-refresh disabled".to_string());
        }
    }

    /// Clear all logs.
    pub fn action_clear_logs(&mut self) {
        self.logs.clear();
        self.notification = Some("Logs cleared".to_string());
    }

    /// Apply a log level filter.
    pub fn apply_filter(&mut self, filter_id: &str) {
        match filter_id {
            "filter_all" => {
                self.active_filter = None;
                self.notification = Some("Showing all logs".to_string());
            },
            "filter_info" => {
                self.active_filter = Some("filter_info".to_string());
                self.notification = Some("Filtered to INFO logs".to_string());
            },
            "filter_warn" => {
                self.active_filter = Some("filter_warn".to_string());
                self.notification = Some("Filtered to WARNING logs".to_string());
            },
            "filter_error" => {
                self.active_filter = Some("filter_error".to_string());
                self.notification = Some("Filtered to ERROR logs".to_string());
            },
            "filter_debug" => {
                self.active_filter = Some("filter_debug".to_string());
                self.notification = Some("Filtered to DEBUG logs".to_string());
            },
            _ => {},
        }
    }

    /// Handle button presses.
    pub fn on_button_pressed(&mut self, button_id: &str) {
        match button_id {
            "follow" => self.action_toggle_follow(),
            "clear" => self.action_clear_logs(),
            "filter_all" | "filter_info" | "filter_warn" | "filter_error" | "filter_debug" => {
                self.apply_filter(button_id);
            },
            _ => {},
        }
    }
}
