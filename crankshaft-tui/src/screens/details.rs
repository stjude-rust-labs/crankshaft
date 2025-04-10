use std::collections::HashMap;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table, Wrap},
    Frame,
};
use serde_json::Value;
use crate::ui::Theme;

/// Details screen for displaying container and service information.
pub struct DetailsScreen {
    pub service_name: String,
    notification: Option<String>,
}

impl DetailsScreen {
    /// Create a new DetailsScreen.
    pub fn new() -> Self {
        DetailsScreen {
            service_name: String::from(""),
            notification: None,
        }
    }

    /// Render the details screen
    pub fn render(&mut self, f: &mut Frame, _data: &Value, buttons: &mut HashMap<String, Rect>, _clickable_areas: &mut HashMap<String, Rect>, theme: &Theme) {
        let size = f.size();
        
        // Outer layout: header, content container, footer
        let outer_chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints(
                [
                    Constraint::Length(3),  // Header
                    Constraint::Min(10),    // Screen container
                    Constraint::Length(3),  // Footer/notification
                ]
                .as_ref(),
            )
            .split(size);

        // Header
        let header = Paragraph::new("SERVICE DETAILS")
            .style(theme.title_style)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(theme.block_style))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(header, outer_chunks[0]);

        // Screen Container
        let container_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(3),  // Subtitle and service name
                    Constraint::Min(10),    // Grid panels
                    Constraint::Length(3),  // Action buttons
                ]
                .as_ref(),
            )
            .split(outer_chunks[1]);

        // Subtitle and service name (Horizontal layout)
        let subtitle_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)].as_ref())
            .split(container_chunks[0]);
            
        let subtitle = Paragraph::new("Container and configuration information")
            .style(theme.text_style)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(theme.block_style));
        f.render_widget(subtitle, subtitle_chunks[0]);
        
        let service_name = Paragraph::new(self.service_name.clone())
            .style(theme.text_style)
            .block(Block::default()
                .title("Service Name")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(theme.block_style));
        f.render_widget(service_name, subtitle_chunks[1]);

        // Grid panels layout
        let grid_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(container_chunks[1]);

        // First row: Configuration and Statistics panels
        let first_row = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(grid_chunks[0]);
            
        // Configuration panel
        let config_text = vec![
            Line::from("Image: nginx:latest"),
            Line::from("Container ID: abc123def456"),
            Line::from("Created: 2024-03-07 08:42:15"),
            Line::from(vec![
                Span::raw("Status: "),
                Span::styled("● Running", Style::default().fg(theme.status_colors.running)),
            ]),
            Line::from("Ports: 8080:80, 443:443"),
            Line::from("Env Vars: NGINX_HOST=example.com, PORT=80"),
        ];
        
        let config_panel = Paragraph::new(Text::from(config_text))
            .style(theme.text_style)
            .block(Block::default()
                .title("CONFIGURATION")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(theme.block_style))
            .wrap(Wrap { trim: true });
        f.render_widget(config_panel, first_row[0]);
        
        // Statistics panel
        let stats_text = vec![
            Line::from(vec![
                Span::raw("CPU Usage: "),
                Span::styled("2.5%", Style::default().fg(Color::Yellow)),
            ]),
            Line::from(vec![
                Span::raw("Memory: "),
                Span::styled("128MB / 512MB", Style::default().fg(Color::Yellow)),
            ]),
            Line::from(vec![
                Span::raw("Network In: "),
                Span::styled("1.2MB/s", Style::default().fg(Color::Blue)),
            ]),
            Line::from(vec![
                Span::raw("Network Out: "),
                Span::styled("0.8MB/s", Style::default().fg(Color::Magenta)),
            ]),
            Line::from(vec![
                Span::raw("Uptime: "),
                Span::styled("2d 3h 45m", Style::default().fg(Color::Green)),
            ]),
            Line::from("Restarts: 0"),
        ];
        
        let stats_panel = Paragraph::new(Text::from(stats_text))
            .style(theme.text_style)
            .block(Block::default()
                .title("STATISTICS")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(theme.block_style))
            .wrap(Wrap { trim: true });
        f.render_widget(stats_panel, first_row[1]);

        // Second row: Volumes and Networks panels
        let second_row = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(grid_chunks[1]);
            
        // Volumes panel
        let volumes_header_cells = vec!["Source", "Destination", "Mode"]
            .into_iter()
            .map(|h| Cell::from(h.to_string()))
            .collect::<Vec<_>>();
            
        let volumes_header = Row::new(volumes_header_cells)
            .style(theme.header_style)
            .height(1).bottom_margin(1);
            
        let volumes_rows = vec![
            vec!["/data", "/var/www/html", "rw"],
            vec!["nginx_logs", "/var/log/nginx", "rw"],
            vec!["nginx_conf", "/etc/nginx/conf.d", "ro"],
        ]
        .into_iter()
        .map(|items| {
            let cells = items.into_iter().map(|c| Cell::from(c.to_string()));
            Row::new(cells).style(theme.text_style).height(1)
        });

        let volumes_table = Table::new(volumes_rows, [Constraint::Percentage(100)])
            .header(volumes_header)
            .block(Block::default()
                .title("VOLUMES")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(theme.block_style))
            .widths(&[
                Constraint::Length(15),
                Constraint::Length(20),
                Constraint::Length(5),
            ]);
        f.render_widget(volumes_table, second_row[0]);
        
        // Networks panel
        let networks_header_cells = vec!["Network", "IP Address", "Gateway"]
            .into_iter()
            .map(|h| Cell::from(h.to_string()))
            .collect::<Vec<_>>();
            
        let networks_header = Row::new(networks_header_cells)
            .style(theme.header_style)
            .height(1).bottom_margin(1);
            
        let networks_rows = vec![
            vec!["bridge", "172.17.0.2", "172.17.0.1"],
            vec!["host", "127.0.0.1", "-"],
        ]
        .into_iter()
        .map(|items| {
            let cells = items.into_iter().map(|c| Cell::from(c.to_string()));
            Row::new(cells).style(theme.text_style).height(1)
        });

        let networks_table = Table::new(networks_rows, [Constraint::Percentage(100)])
            .header(networks_header)
            .block(Block::default()
                .title("NETWORKS")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(theme.block_style))
            .widths(&[
                Constraint::Length(10),
                Constraint::Length(15),
                Constraint::Length(10),
            ]);
        f.render_widget(networks_table, second_row[1]);

        // Action buttons
        let button_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
            ].as_ref())
            .split(container_chunks[2]);
        
        // Logs button
        let logs_button = Paragraph::new("View Logs")
            .style(theme.button_style)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded));
        f.render_widget(logs_button, button_chunks[0]);
        buttons.insert("details_logs".to_string(), button_chunks[0]);
        
        // Restart button
        let restart_button = Paragraph::new("Restart")
            .style(theme.button_style)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded));
        f.render_widget(restart_button, button_chunks[1]);
        buttons.insert("details_restart".to_string(), button_chunks[1]);
        
        // Stop button
        let stop_button = Paragraph::new("Stop")
            .style(theme.button_style)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded));
        f.render_widget(stop_button, button_chunks[2]);
        buttons.insert("details_stop".to_string(), button_chunks[2]);
        
        // Back button
        let back_button = Paragraph::new("Back")
            .style(theme.button_style)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded));
        f.render_widget(back_button, button_chunks[3]);
        buttons.insert("back".to_string(), button_chunks[3]);

        // Footer with notification or key bindings
        let footer_text = match &self.notification {
            Some(msg) => format!("NOTIFY: {}", msg),
            None => "ESC: Back | r: Refresh | l: View Logs".to_string(),
        };
        
        let footer = Paragraph::new(footer_text)
            .style(theme.text_style)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(theme.block_style))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(footer, outer_chunks[2]);
    }

    /// Handle mounting of the screen.
    pub fn on_mount(&mut self) {
        self.service_name = String::from("nginx");
        self.notification = Some("Details screen loaded".to_string());
    }

    /// Refresh the details data.
    pub fn action_refresh(&self) {
        println!("Refreshing service details...");
    }

    /// Handle button presses.
    pub fn on_button_pressed(&mut self, button_id: &str) {
        match button_id {
            "details_logs" => {
                self.notification = Some("Viewing logs...".to_string());
            },
            "details_restart" => {
                self.notification = Some("Restarting service...".to_string());
            },
            "details_stop" => {
                self.notification = Some("Stopping service...".to_string());
            },
            "back" => {
                self.notification = Some("Going back...".to_string());
            },
            _ => {}
        }
    }
}
