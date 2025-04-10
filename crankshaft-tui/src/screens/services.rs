use std::collections::HashMap;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table, Wrap},
    Frame,
};
use serde_json::Value;
use crate::ui::Theme;

/// Services management screen for interacting with running containers.
pub struct ServiceScreen {
    /// Table data representing services.
    table_data: Vec<[String; 5]>,
    /// Currently selected row index.
    selected_row: Option<usize>,
    /// Service details (name, status, ports, image, container_id).
    details: Option<[String; 5]>,
    /// Last notification message.
    notification: Option<String>,
}

impl ServiceScreen {
    /// Create a new ServiceScreen.
    pub fn new() -> Self {
        // This would be populated from actual Crankshaft config in the future
        let table_data = vec![
            [
                "frontend".to_string(),
                "● Running".to_string(),
                "3000:3000".to_string(),
                "node:16".to_string(),
                "abc123def456".to_string(),
            ],
            [
                "api".to_string(),
                "● Running".to_string(),
                "8000:8000".to_string(),
                "python:3.10".to_string(),
                "def456ghi789".to_string(),
            ],
            [
                "db".to_string(),
                "● Running".to_string(),
                "5432:5432".to_string(),
                "postgres:14".to_string(),
                "789ghi101112".to_string(),
            ],
            [
                "redis".to_string(),
                "● Running".to_string(),
                "6379:6379".to_string(),
                "redis:latest".to_string(),
                "jkl131415mno".to_string(),
            ],
            [
                "nginx".to_string(),
                "● Running".to_string(),
                "80:80, 443:443".to_string(),
                "nginx:latest".to_string(),
                "pqr161718stu".to_string(),
            ],
            [
                "worker".to_string(),
                "○ Stopped".to_string(),
                "-".to_string(),
                "python:3.10".to_string(),
                "".to_string(),
            ],
        ];
        // On mount, select the first row and update details
        let mut details = None;
        if !table_data.is_empty() {
            details = Some(table_data[0].clone());
        }
        Self {
            table_data,
            selected_row: Some(0),
            details,
            notification: Some("Services screen loaded".to_string()),
        }
    }

    /// Handle mounting of the screen.
    pub fn on_mount(&mut self) {
        // Initialize the screen
        self.notification = Some("Service screen loaded".to_string());
        
        // Make sure a service is selected
        if let Some(idx) = self.selected_row {
            if idx < self.table_data.len() {
                self.details = Some(self.table_data[idx].clone());
            }
        }
    }

    /// Render the services screen
    pub fn render(&mut self, f: &mut Frame, _data: &Value, buttons: &mut HashMap<String, Rect>, clickable_areas: &mut HashMap<String, Rect>, theme: &Theme) {
        let size = f.size();
        
        // Split the main frame into header, content and footer
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(3), // Header
                    Constraint::Min(10),   // Main content
                    Constraint::Length(3), // Footer / message
                ]
                .as_ref(),
            )
            .split(size);

        // Header
        let header = Paragraph::new("SERVICE MANAGEMENT")
            .style(theme.title_style)
            .block(Block::default()
                .title("Container Services")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(theme.block_style))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(header, chunks[0]);

        // Main Content Container
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(3), // Subtitles / screen header
                    Constraint::Min(9),    // Main table area
                    Constraint::Length(3), // Actions buttons area
                ]
                .as_ref(),
            )
            .split(chunks[1]);

        // Screen Header
        let header_text = vec![
            Line::from("Manage your container services"),
            Line::from(Span::styled(
                format!("Total Services: {}", self.table_data.len()),
                Style::default().add_modifier(ratatui::style::Modifier::BOLD),
            )),
        ];
        
        let header_para = Paragraph::new(Text::from(header_text))
            .style(theme.text_style)
            .block(Block::default()
                .title("Services")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(theme.block_style));
        f.render_widget(header_para, main_chunks[0]);

        // Services table
        let header_cells = vec!["Service", "Status", "Ports", "Image", "Container ID"]
            .into_iter()
            .map(|h| h.clone())
            .collect::<Vec<_>>();
            
        let header = Row::new(header_cells)
            .style(theme.header_style)
            .height(1);
            
        let rows = self.table_data.iter().enumerate().map(|(i, row)| {
            let row_style = if Some(i) == self.selected_row {
                theme.selected_style
            } else {
                theme.text_style
            };
            
            // Use cell styling just for the status column
            let mut cells = Vec::new();
            for (j, cell) in row.iter().enumerate() {
                if j == 1 {  // Status column
                    let status_style = if cell.contains("Running") {
                        Style::default().fg(theme.status_colors.running)
                    } else {
                        Style::default().fg(theme.status_colors.stopped)
                    };
                    cells.push(Cell::from(cell.clone()).style(status_style));
                } else {
                    cells.push(Cell::from(cell.clone()));
                }
            }
            
            Row::new(cells).style(row_style).height(1)
        });

        let table = Table::new(rows, [Constraint::Percentage(100)])
            .header(header)
            .block(Block::default()
                .title("RUNNING SERVICES")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(theme.block_style))
            .widths(&[
                Constraint::Percentage(20),
                Constraint::Percentage(15),
                Constraint::Percentage(15),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
            ])
            .column_spacing(1);
            
        f.render_widget(table, main_chunks[1]);
        
        // Save the table area for clickable detection
        clickable_areas.insert("services_table".to_string(), main_chunks[1]);

        // Service Details if available
        if let Some(detail) = &self.details {
            let detail_lines = vec![
                Line::from(vec![
                    Span::styled("Name: ", Style::default().add_modifier(ratatui::style::Modifier::BOLD)),
                    Span::raw(detail[0].clone()),
                ]),
                Line::from(vec![
                    Span::styled("Status: ", Style::default().add_modifier(ratatui::style::Modifier::BOLD)),
                    Span::raw(detail[1].clone()),
                ]),
                Line::from(vec![
                    Span::styled("Ports: ", Style::default().add_modifier(ratatui::style::Modifier::BOLD)),
                    Span::raw(detail[2].clone()),
                ]),
                Line::from(vec![
                    Span::styled("Image: ", Style::default().add_modifier(ratatui::style::Modifier::BOLD)),
                    Span::raw(detail[3].clone()),
                ]),
                Line::from(vec![
                    Span::styled("Container ID: ", Style::default().add_modifier(ratatui::style::Modifier::BOLD)),
                    Span::raw(detail[4].clone()),
                ]),
            ];
                
            let details_block = Block::default()
                .title("SERVICE DETAILS")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(theme.block_style);
                
            let details_para = Paragraph::new(Text::from(detail_lines))
                .style(theme.text_style)
                .block(details_block)
                .wrap(Wrap { trim: true });
                
            // Use a floating popup for details
            let details_area = Rect {
                x: size.x + (size.width / 4),
                y: size.y + (size.height / 4),
                width: size.width / 2,
                height: 8,
            };
            
            f.render_widget(details_para, details_area);
        }

        // Action buttons
        let action_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
            ].as_ref())
            .split(main_chunks[2]);
            
        // Start button
        let start_button = Paragraph::new("Start")
            .style(theme.button_style)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(start_button, action_chunks[0]);
        buttons.insert("start".to_string(), action_chunks[0]);
        
        // Stop button
        let stop_button = Paragraph::new("Stop")
            .style(theme.button_style)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(stop_button, action_chunks[1]);
        buttons.insert("stop".to_string(), action_chunks[1]);
        
        // Restart button
        let restart_button = Paragraph::new("Restart")
            .style(theme.button_style)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(restart_button, action_chunks[2]);
        buttons.insert("restart".to_string(), action_chunks[2]);
        
        // Back button
        let back_button = Paragraph::new("Back")
            .style(theme.button_style)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(back_button, action_chunks[3]);
        buttons.insert("back".to_string(), action_chunks[3]);

        // Footer
        let footer_text = match &self.notification {
            Some(msg) => format!("NOTIFY: {}", msg),
            None => "Press 1:Start 2:Stop 3:Restart | ESC: Back | Up/Down: Navigate".to_string(),
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

    /// Update the selected row index.
    pub fn select_row(&mut self, new_index: usize) {
        if new_index < self.table_data.len() {
            self.selected_row = Some(new_index);
            self.details = Some(self.table_data[new_index].clone());
            self.notification = Some(format!("Selected service: {}", self.table_data[new_index][0]));
        }
    }

    /// Move selection up.
    pub fn select_up(&mut self) {
        if let Some(current) = self.selected_row {
            let new_index = if current == 0 {
                self.table_data.len() - 1
            } else {
                current - 1
            };
            self.select_row(new_index);
        }
    }

    /// Move selection down.
    pub fn select_down(&mut self) {
        if let Some(current) = self.selected_row {
            let new_index = if current >= self.table_data.len() - 1 {
                0
            } else {
                current + 1
            };
            self.select_row(new_index);
        }
    }

    /// Refresh the services data.
    pub fn action_refresh(&mut self) {
        self.notification = Some("Refreshing services data...".to_string());
    }

    /// Handle service action (start/stop/restart).
    pub fn perform_action(&mut self, action: &str) {
        if let Some(idx) = self.selected_row {
            let service = &self.table_data[idx][0];
            self.notification = Some(format!("{}ing {}...", action, service));
            
            // In a real implementation, we would make API calls to control the service
            // For now, we'll update the status in our mock data
            if action.to_lowercase() == "start" {
                self.table_data[idx][1] = "● Running".to_string();
            } else if action.to_lowercase() == "stop" {
                self.table_data[idx][1] = "○ Stopped".to_string();
            }
            
            // Update details
            self.details = Some(self.table_data[idx].clone());
        } else {
            self.notification = Some("Please select a service first".to_string());
        }
    }

    /// Handle button press events.
    pub fn on_button_pressed(&mut self, button_id: &str) {
        match button_id {
            "start" => self.perform_action("Start"),
            "stop" => self.perform_action("Stop"),
            "restart" => self.perform_action("Restart"),
            "back" => self.notification = Some("Going back to main screen...".to_string()),
            _ => {}
        }
    }
}
