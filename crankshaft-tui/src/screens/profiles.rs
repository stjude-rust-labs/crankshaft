use std::collections::HashMap;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, BorderType, Borders, Paragraph, Row, Table, TableState},
    Frame,
};
use serde_json::Value;
use crate::ui::Theme;

/// Profiles management screen for Crankshaft TUI.
pub struct ProfileScreen {
    table_data: Vec<Vec<String>>,
    table_state: TableState,
    details: Option<ProfileDetails>,
    notification: Option<String>,
}

/// Structure to hold profile details.
#[derive(Clone)]
struct ProfileDetails {
    name: String,
    project: String,
    config: String,
    services: String,
}

impl ProfileScreen {
    /// Create a new ProfileScreen.
    pub fn new() -> Self {
        // Initialize mock table data
        let table_data = vec![
            vec![
                "development".to_string(),
                "project-dev".to_string(),
                "containers/dev/compose.yaml".to_string(),
                "frontend, api, db".to_string(),
                "● Active".to_string(),
            ],
            vec![
                "staging".to_string(),
                "project-staging".to_string(),
                "containers/staging/compose.yaml".to_string(),
                "All services".to_string(),
                "● Active".to_string(),
            ],
            vec![
                "production".to_string(),
                "project-prod".to_string(),
                "containers/prod/compose.yaml".to_string(),
                "web, api, db, cache".to_string(),
                "○ Inactive".to_string(),
            ],
            vec![
                "testing".to_string(),
                "project-test".to_string(),
                "containers/test/compose.yaml".to_string(),
                "test-suite, mockdb".to_string(),
                "○ Inactive".to_string(),
            ],
        ];
        let mut table_state = TableState::default();
        table_state.select(Some(0));

        ProfileScreen {
            table_data,
            table_state,
            details: None,
            notification: None,
        }
    }

    /// Render the profile screen
    pub fn render(&mut self, f: &mut Frame, _data: &Value, buttons: &mut HashMap<String, Rect>, clickable_areas: &mut HashMap<String, Rect>, theme: &Theme) {
        let size = f.size();
        
        // Split the main area into header, content, and footer areas.
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(3), // Header
                    Constraint::Min(5),    // Content
                    Constraint::Length(3), // Footer
                ]
                .as_ref(),
            )
            .split(size);

        // Header
        let header = Paragraph::new("PROFILE MANAGEMENT")
            .style(theme.title_style)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(theme.block_style))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(header, chunks[0]);

        // Main container
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(3), // Subtitle area
                    Constraint::Min(10),   // Main table area
                    Constraint::Length(3), // Profile action buttons
                ]
                .as_ref(),
            )
            .split(chunks[1]);

        // Subtitle area with two columns
        let subtitle_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(70), Constraint::Percentage(30)].as_ref())
                .split(main_chunks[0]);
            
            let left_subtitle = Paragraph::new("Manage your Crankshaft profiles")
                .style(theme.text_style)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(theme.block_style));
            f.render_widget(left_subtitle, subtitle_chunks[0]);
            
            let right_subtitle = Paragraph::new(format!("Total Profiles: {}", self.table_data.len()))
                .style(theme.text_style)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(theme.block_style))
                .alignment(ratatui::layout::Alignment::Right);
            f.render_widget(right_subtitle, subtitle_chunks[1]);
    
            // Profiles table
            let selected = self.table_state.selected();
                
            // Create header cells
            let header_cells = vec!["Profile", "Project Name", "Config Path", "Services", "Status"]
                .into_iter()
                .map(|h| h.clone())
                .collect::<Vec<_>>();
                
            let header = Row::new(header_cells)
                .style(theme.header_style)
                .height(1).bottom_margin(1);
    
            // Create rows from table_data
            let rows = self.table_data.iter().enumerate().map(|(i, item)| {
                let row_style = if Some(i) == selected {
                    theme.selected_style
                } else {
                    theme.text_style
                };
                
                Row::new(item.clone()).style(row_style).height(1)
            });
    
            let profiles_table = Table::new(rows, [Constraint::Percentage(100)])
                .header(header)
                .block(Block::default()
                    .title("PROFILES")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(theme.block_style))
                .widths(&[
                    Constraint::Percentage(20),
                    Constraint::Percentage(20),
                    Constraint::Percentage(20),
                    Constraint::Percentage(20),
                    Constraint::Percentage(20),
                ])
                .highlight_style(theme.highlight_style)
                .column_spacing(1);
                
            // Save the table area for clickable detection
            let table_area = main_chunks[1];
            clickable_areas.insert("profiles_table".to_string(), table_area);
                
            f.render_stateful_widget(profiles_table, main_chunks[1], &mut self.table_state);
    
            // Profile action buttons
            let button_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(25),
                    Constraint::Percentage(25),
                    Constraint::Percentage(25),
                    Constraint::Percentage(25),
                ].as_ref())
                .split(main_chunks[2]);
                
            // Add profile button
            let add_button = Paragraph::new("Add Profile")
                .style(theme.button_style)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded))
                .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(add_button, button_chunks[0]);
            buttons.insert("add_profile".to_string(), button_chunks[0]);
            
            // Edit profile button
            let edit_button = Paragraph::new("Edit Profile")
                .style(theme.button_style)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded))
                .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(edit_button, button_chunks[1]);
            buttons.insert("edit_profile".to_string(), button_chunks[1]);
            
            // Delete profile button
            let delete_button = Paragraph::new("Delete Profile")
                .style(theme.button_style)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded))
                .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(delete_button, button_chunks[2]);
            buttons.insert("delete_profile".to_string(), button_chunks[2]);
            
            // Back button
            let back_button = Paragraph::new("Back")
                .style(theme.button_style)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded))
                .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(back_button, button_chunks[3]);
            buttons.insert("back".to_string(), button_chunks[3]);
    
            // Footer
            let footer_text = match &self.notification {
                Some(msg) => format!("NOTIFY: {}", msg),
                None => "Press Up/Down to navigate, Enter to select, ESC to go back".to_string(),
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
            // Initialize the details with the first profile
            if !self.table_data.is_empty() {
                let row = self.table_data[0].clone();
                self.details = Some(ProfileDetails {
                    name: row[0].clone(),
                    project: row[1].clone(),
                    config: row[2].clone(),
                    services: row[3].clone(),
                });
            }
            self.notification = Some("Profile screen loaded".to_string());
        }
    
        /// Update the profile details when a row is selected.
        pub fn update_selection(&mut self) {
            if let Some(selected) = self.table_state.selected() {
                if selected < self.table_data.len() {
                    let row = self.table_data[selected].clone();
                    self.details = Some(ProfileDetails {
                        name: row[0].clone(),
                        project: row[1].clone(),
                        config: row[2].clone(),
                        services: row[3].clone(),
                    });
                    
                    let profile_name = &row[0];
                    self.notification = Some(format!("Selected profile: {}", profile_name));
                }
            }
        }
    
        /// Select a specific row
        pub fn select_row(&mut self, index: usize) {
            if index < self.table_data.len() {
                self.table_state.select(Some(index));
                self.update_selection();
            }
        }
    
        /// Refresh the profiles data.
        pub fn action_refresh(&mut self) {
            self.notification = Some("Refreshing profiles data...".to_string());
        }
    
        /// Handle button presses.
        pub fn on_button_pressed(&mut self, button_id: &str) {
            match button_id {
                "add_profile" => {
                    self.notification = Some("Adding new profile...".to_string());
                },
                "edit_profile" => {
                    if let Some(idx) = self.table_state.selected() {
                        let profile = &self.table_data[idx][0];
                        self.notification = Some(format!("Editing profile: {}", profile));
                    } else {
                        self.notification = Some("Please select a profile first".to_string());
                    }
                },
                "delete_profile" => {
                    if let Some(idx) = self.table_state.selected() {
                        let profile = &self.table_data[idx][0];
                        self.notification = Some(format!("Deleting profile: {}", profile));
                    } else {
                        self.notification = Some("Please select a profile first".to_string());
                    }
                },
                "back" => {
                    self.notification = Some("Going back to main screen...".to_string());
                },
                _ => {}
            }
        }
    
        /// Move the selection up in the table.
        pub fn select_up(&mut self) {
            let i = match self.table_state.selected() {
                Some(i) => {
                    if i == 0 {
                        self.table_data.len() - 1
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.table_state.select(Some(i));
            self.update_selection();
        }
    
        /// Move the selection down in the table.
        pub fn select_down(&mut self) {
            let i = match self.table_state.selected() {
                Some(i) => {
                    if i >= self.table_data.len() - 1 {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            self.table_state.select(Some(i));
            self.update_selection();
        }
    }
    