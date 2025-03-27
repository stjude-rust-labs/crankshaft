use std::collections::HashMap;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table, Wrap},
    Frame,
};
use serde_json::Value;

use crate::screens::{
    details::DetailsScreen,
    logs::LogsScreen,
    profiles::ProfileScreen,
    services::ServiceScreen
};
use crate::ui::Theme;
use crate::app::CrankshaftTUI;

/// Render the main dashboard screen
pub fn render_main_screen(
    f: &mut Frame,
    data: &Value,
    theme: &Theme,
    buttons: &mut HashMap<String, Rect>,
    clickable_areas: &mut HashMap<String, Rect>,
    hover_element: Option<&str>
) {
    let size = f.size();
    
    // Create main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(3),  // Header
                Constraint::Min(10),    // Body
                Constraint::Length(3),  // Footer
            ]
            .as_ref(),
        )
        .split(size);

    // Header
    let header = Paragraph::new(CrankshaftTUI::TITLE)
        .style(theme.app_title_style)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(theme.block_style))
        .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(header, chunks[0]);

    // Body
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
            ]
            .as_ref(),
        )
        .split(chunks[1]);

    // Profiles Panel
    render_profiles_panel(f, data, theme, body_chunks[0], clickable_areas);

    // Services Panel
    render_services_panel(f, data, theme, body_chunks[1], clickable_areas);

    // System Metrics Panel
    render_status_panel(f, data, theme, body_chunks[2]);

    // Quick Actions Panel (with buttons)
    render_actions_panel(f, theme, body_chunks[3], buttons, hover_element);

    // Footer
    let footer = Paragraph::new("q: Quit | p: Profiles | s: Services | l: Logs | d: Details | r: Refresh | Esc: Back")
        .style(theme.text_style)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(theme.block_style))
        .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(footer, chunks[2]);
}

fn render_profiles_panel(
    f: &mut Frame, 
    data: &Value, 
    theme: &Theme, 
    area: Rect,
    clickable_areas: &mut HashMap<String, Rect>
) {
    let header_cells = vec!["Profile", "Services", "Status"]
        .into_iter()
        .map(|s| Cell::from(s.to_string()))
        .collect::<Vec<_>>();
        
    let header = Row::new(header_cells)
        .style(theme.header_style)
        .height(1).bottom_margin(1);
    
    let mut rows = Vec::new();

    if let Some(profiles) = data.get("profiles").and_then(|v| v.as_array()) {
        for profile in profiles {
            let profile_name = profile.get("profile").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let services = profile.get("services").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let status = profile.get("status").and_then(|v| v.as_str()).unwrap_or("").to_string();
            
            let status_style = if status.contains("Active") {
                Style::default().fg(theme.status_colors.active)
            } else {
                Style::default().fg(theme.status_colors.inactive)
            };
            
            let cells = vec![
                Cell::from(profile_name),
                Cell::from(services),
                Cell::from(status).style(status_style),
            ];
            
            let row = Row::new(cells).height(1);
            rows.push(row);
        }
    }

    let profiles_table = Table::new(rows, [Constraint::Percentage(100)])
        .header(header)
        .block(Block::default()
            .title("ACTIVE PROFILES")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(theme.block_style))
        .widths(&[
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ])
        .column_spacing(1);
        
    f.render_widget(profiles_table, area);
    
    // Register clickable area for the table
    clickable_areas.insert(
        "table_profiles".to_string(), 
        Rect {
            x: area.x + 1,
            y: area.y + 2,
            width: area.width - 2,
            height: area.height - 3,
        }
    );
}

fn render_services_panel(
    f: &mut Frame, 
    data: &Value, 
    theme: &Theme, 
    area: Rect,
    clickable_areas: &mut HashMap<String, Rect>
) {
    let header_cells = vec!["Service", "Status", "Ports", "CPU", "Memory"]
        .into_iter()
        .map(|s| Cell::from(s.to_string()))
        .collect::<Vec<_>>();
        
    let header = Row::new(header_cells)
        .style(theme.header_style)
        .height(1).bottom_margin(1);
        
    let mut rows = Vec::new();

    if let Some(services) = data.get("services").and_then(|v| v.as_array()) {
        for service in services {
            let service_name = service.get("service").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let status = service.get("status").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let ports = service.get("ports").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let cpu = service.get("cpu").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let memory = service.get("memory").and_then(|v| v.as_str()).unwrap_or("").to_string();
            
            let status_style = if status.contains("Running") {
                Style::default().fg(theme.status_colors.running)
            } else {
                Style::default().fg(theme.status_colors.stopped)
            };
            
            let cells = vec![
                Cell::from(service_name),
                Cell::from(status).style(status_style),
                Cell::from(ports),
                Cell::from(cpu),
                Cell::from(memory),
            ];
            
            let row = Row::new(cells).height(1);
            rows.push(row);
        }
    }

    let services_table = Table::new(rows, [Constraint::Percentage(100)])
        .header(header)
        .block(Block::default()
            .title("RUNNING SERVICES")
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
        .column_spacing(1);
        
    f.render_widget(services_table, area);
    
    // Register clickable area for the table
    clickable_areas.insert(
        "table_services".to_string(), 
        Rect {
            x: area.x + 1,
            y: area.y + 2,
            width: area.width - 2,
            height: area.height - 3,
        }
    );
}

fn render_status_panel(f: &mut Frame, data: &Value, theme: &Theme, area: Rect) {
    let metrics = data.get("system_metrics").cloned().unwrap_or_else(|| serde_json::json!({}));

    let active_profiles = metrics.get("active_profiles").and_then(|v| v.as_str()).unwrap_or("N/A").to_string();
    let running_containers = metrics.get("running_containers").and_then(|v| v.as_str()).unwrap_or("N/A").to_string();
    let cpu_usage = metrics.get("cpu_usage").and_then(|v| v.as_str()).unwrap_or("N/A").to_string();
    let memory_usage = metrics.get("memory_usage").and_then(|v| v.as_str()).unwrap_or("N/A").to_string();
    let disk_usage = metrics.get("disk_usage").and_then(|v| v.as_str()).unwrap_or("N/A").to_string();
    let network_in = metrics.get("network_in").and_then(|v| v.as_str()).unwrap_or("N/A").to_string();
    let network_out = metrics.get("network_out").and_then(|v| v.as_str()).unwrap_or("N/A").to_string();
    let uptime = metrics.get("uptime").and_then(|v| v.as_str()).unwrap_or("N/A").to_string();

    let lines = vec![
        Line::from(vec![
            Span::raw("Active Profiles: "),
            Span::styled(active_profiles, Style::default().fg(theme.status_colors.active)),
        ]),
        Line::from(vec![
            Span::raw("Running Containers: "),
            Span::styled(running_containers, Style::default().fg(theme.status_colors.running)),
        ]),
        Line::from("──────────────────────"),
        Line::from(vec![
            Span::raw("CPU Usage: "),
            Span::styled(cpu_usage, Style::default().fg(Color::Red)),
        ]),
        Line::from(vec![
            Span::raw("Memory Usage: "),
            Span::styled(memory_usage, Style::default().fg(Color::Red)),
        ]),
        Line::from(vec![
            Span::raw("Disk Usage: "),
            Span::styled(disk_usage, Style::default().fg(Color::Green)),
        ]),
        Line::from("──────────────────────"),
        Line::from(vec![
            Span::raw("Network In: "),
            Span::styled(network_in, Style::default().fg(Color::Blue)),
        ]),
        Line::from(vec![
            Span::raw("Network Out: "),
            Span::styled(network_out, Style::default().fg(Color::Magenta)),
        ]),
        Line::from("──────────────────────"),
        Line::from(vec![
            Span::raw("Uptime: "),
            Span::styled(uptime, Style::default().fg(Color::Green)),
        ]),
    ];

    let status_panel = Paragraph::new(Text::from(lines))
        .block(Block::default()
            .title("SYSTEM METRICS")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(theme.block_style))
        .wrap(Wrap { trim: true });
        
    f.render_widget(status_panel, area);
}

fn render_actions_panel(
    f: &mut Frame, 
    theme: &Theme, 
    area: Rect, 
    buttons: &mut HashMap<String, Rect>,
    hover_element: Option<&str>
) {
    // Title block
    let block = Block::default()
        .title("QUICK ACTIONS")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme.block_style);
        
    f.render_widget(block, area);
    
    // Create inner area for buttons
    let inner_area = Rect {
        x: area.x + 2,
        y: area.y + 2,
        width: area.width - 4,
        height: area.height - 4,
    };
    
    // Layout for buttons
    let button_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
        ].as_ref())
        .margin(1)
        .split(inner_area);
        
    // Define buttons with hover effect
    let button_configs = [
        ("Start All", "start_all", button_chunks[0]),
        ("Stop All", "stop_all", button_chunks[1]),
        ("Restart", "restart_all", button_chunks[2]),
        ("View Logs", "view_logs", button_chunks[3]),
        ("Details", "service_details", button_chunks[4]),
        ("Health Check", "health_checks", button_chunks[5]),
    ];
    
    for (text, id, rect) in button_configs {
        let is_hover = hover_element.map_or(false, |e| e == id);
        let button_style = if is_hover {
            theme.button_hover_style
        } else {
            theme.button_style
        };
        
        let button = Paragraph::new(text)
            .style(button_style)
            .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded))
            .alignment(ratatui::layout::Alignment::Center);
            
        f.render_widget(button, rect);
        
        // Register this button
        buttons.insert(id.to_string(), rect);
    }
}

/// Render the details screen
pub fn render_details_screen(
    f: &mut Frame,
    screen: &mut DetailsScreen,
    data: &Value,
    theme: &Theme,
    buttons: &mut HashMap<String, Rect>,
    clickable_areas: &mut HashMap<String, Rect>,
) {
    screen.render(f, data, buttons, clickable_areas, theme);
}

/// Render the logs screen
pub fn render_logs_screen(
    f: &mut Frame,
    screen: &mut LogsScreen,
    data: &Value,
    theme: &Theme,
    buttons: &mut HashMap<String, Rect>,
    clickable_areas: &mut HashMap<String, Rect>,
) {
    screen.render(f, data, buttons, clickable_areas, theme);
}

/// Render the profiles screen
pub fn render_profiles_screen(
    f: &mut Frame,
    screen: &mut ProfileScreen,
    data: &Value,
    theme: &Theme,
    buttons: &mut HashMap<String, Rect>,
    clickable_areas: &mut HashMap<String, Rect>,
) {
    screen.render(f, data, buttons, clickable_areas, theme);
}

/// Render the services screen
pub fn render_services_screen(
    f: &mut Frame,
    screen: &mut ServiceScreen,
    data: &Value,
    theme: &Theme,
    buttons: &mut HashMap<String, Rect>,
    clickable_areas: &mut HashMap<String, Rect>,
) {
    screen.render(f, data, buttons, clickable_areas, theme);
}
