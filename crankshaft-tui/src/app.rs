use std::collections::HashMap;
use std::fs;
use std::io::{self, Stdout};
use std::path::{PathBuf};
use std::time::Duration;

use crossterm::{
    event::{self, EnableMouseCapture, DisableMouseCapture, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use ratatui::{
    backend::CrosstermBackend,
    layout::Rect,
    Terminal,
};
use serde_json::{json, Value};

use crate::screens::{
    details::DetailsScreen,
    logs::LogsScreen,
    profiles::ProfileScreen,
    services::ServiceScreen
};
use crate::ui::Theme;
use crate::renderer;


pub mod crankshaft_logs {
    pub fn print_info(message: &str) {
        println!("[INFO] {}", message);
    }
    
    pub fn print_warning(message: &str) {
        println!("[WARNING] {}", message);
    }
}

fn get_manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[derive(Debug, Clone, Copy)]
pub enum MouseAction {
    Clicked,
    Pressed,
    Released,
    Moved,
    ScrollUp,
    ScrollDown,
}

pub struct CrankshaftTUI {
    mock_data: Value,
    terminal: Terminal<CrosstermBackend<Stdout>>,
    current_screen: String,
    details_screen: Option<DetailsScreen>,
    logs_screen: Option<LogsScreen>,
    profiles_screen: Option<ProfileScreen>,
    services_screen: Option<ServiceScreen>,
    theme: Theme,
    hover_element: Option<String>,
    buttons: HashMap<String, Rect>,
    clickable_areas: HashMap<String, Rect>,
}

impl CrankshaftTUI {
    pub const TITLE: &'static str = "Crankshaft TUI — Container Management Simplified";

    pub fn css_path() -> PathBuf {
        get_manifest_dir().join("styles").join("styles.css")
    }
    
    pub fn data_path() -> PathBuf {
        get_manifest_dir().join("src").join("data").join("app.json")
    }

    pub fn bindings() -> Vec<(&'static str, &'static str, &'static str)> {
        vec![
            ("q", "quit", "Quit"),
            ("p", "push_screen('profiles')", "Profiles"),
            ("s", "push_screen('services')", "Services"),
            ("l", "logs", "Logs"),
            ("d", "details", "Details"),
            ("r", "refresh", "Refresh"),
            ("escape", "back", "Back"),
        ]
    }

    pub fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        
        let mut tui = CrankshaftTUI {
            mock_data: json!({}),
            terminal,
            current_screen: "main".to_string(),
            details_screen: None,
            logs_screen: None,
            profiles_screen: None,
            services_screen: None,
            theme: Theme::default(),
            hover_element: None,
            buttons: HashMap::new(),
            clickable_areas: HashMap::new(),
        };
        tui.mock_data = tui.load_mock_data();
        Ok(tui)
    }

    pub fn load_mock_data(&self) -> Value {
        match fs::read_to_string(Self::data_path()) {
            Ok(content) => match serde_json::from_str::<Value>(&content) {
                Ok(data) => data,
                Err(e) => {
                    crankshaft_logs::print_warning(&format!("Error loading mock data: {}", e));
                    json!({"profiles": [], "services": [], "system_metrics": {}})
                }
            },
            Err(e) => {
                crankshaft_logs::print_warning(&format!("Error loading mock data: {}", e));
                json!({"profiles": [], "services": [], "system_metrics": {}})
            }
        }
    }

    pub fn on_mount(&mut self) {
        let install_result: Result<(), Box<dyn std::error::Error>> = (|| {
            self.current_screen = "main".to_string();
            Ok(())
        })();

        match install_result {
            Ok(_) => {
                crankshaft_logs::print_info("Screens installed successfully");
            }
            Err(e) => {
                crankshaft_logs::print_warning(&format!("Error loading screens: {}", e));
                self._create_basic_screens();
            }
        }

        // Initialize all screens
        self.details_screen = Some(DetailsScreen::new());
        self.logs_screen = Some(LogsScreen::new());
        self.profiles_screen = Some(ProfileScreen::new());
        self.services_screen = Some(ServiceScreen::new());

        // Register clickable areas
        self.register_clickable_areas();
    }

    fn register_clickable_areas(&mut self) {
        // Clear existing areas
        self.buttons.clear();
        self.clickable_areas.clear();

        // These will be populated when rendering
    }

    pub fn register_button(&mut self, id: &str, area: Rect) {
        self.buttons.insert(id.to_string(), area);
    }

    pub fn register_clickable(&mut self, id: &str, area: Rect) {
        self.clickable_areas.insert(id.to_string(), area);
    }

    pub fn action_refresh(&mut self) {
        self.notify("Refreshing dashboard data...", None, None);
        self.mock_data = self.load_mock_data();
    }

    pub fn action_logs(&mut self) {
        self.notify("Viewing logs...", None, None);
        if self.screen_available("logs") {
            if self.logs_screen.is_none() {
                self.logs_screen = Some(LogsScreen::new());
            }
            
            self.current_screen = "logs".to_string();
            
            if let Some(screen) = &mut self.logs_screen {
                screen.on_mount();
            }
        } else {
            self.notify("Logs screen not available", None, Some("error"));
        }
    }

    pub fn action_details(&mut self) {
        self.notify("Viewing details...", None, None);
        if self.screen_available("details") {
            if self.details_screen.is_none() {
                self.details_screen = Some(DetailsScreen::new());
            }
            
            self.current_screen = "details".to_string();
            
            if let Some(screen) = &mut self.details_screen {
                screen.on_mount();
                screen.service_name = "selected-service".to_string();
            }
        } else {
            self.notify("Details screen not available", None, Some("error"));
        }
    }

    pub fn switch_to_profiles(&mut self) {
        if self.profiles_screen.is_none() {
            self.profiles_screen = Some(ProfileScreen::new());
        }
        if let Some(screen) = &mut self.profiles_screen {
            screen.on_mount();
        }
        self.current_screen = "profiles".to_string();
    }
    
    pub fn switch_to_services(&mut self) {
        if self.services_screen.is_none() {
            self.services_screen = Some(ServiceScreen::new());
        }
        if let Some(screen) = &mut self.services_screen {
            screen.on_mount();
        }
        self.current_screen = "services".to_string();
    }
    
    pub fn action_back(&mut self) {
        if self.current_screen != "main" {
            self.current_screen = "main".to_string();
        } else {
            println!("[DEBUG] Could not pop screen: already at main screen");
        }
    }

    pub fn on_button_pressed(&mut self, button_text: &str) {
        if button_text.contains("Start") {
            self.notify("Starting all services...", Some("Starting"), None);
        } else if button_text.contains("Stop") {
            self.notify("Stopping all services...", Some("Stopping"), None);
        } else if button_text.contains("Restart") {
            self.notify("Restarting all services...", Some("Restarting"), None);
        } else if button_text.contains("Logs") {
            self.action_logs();
        } else if button_text.contains("Details") {
            self.action_details();
        } else if button_text.contains("Health") {
            self.notify("Running health checks...", Some("Health Check"), None);
        } else if button_text == "Back" {
            self.action_back();
        } else if button_text == "Profiles" {
            self.switch_to_profiles();
        } else if button_text == "Services" {
            self.switch_to_services();
        }
    }

    pub fn handle_button_click(&mut self, button_id: &str) {
        match button_id {
            "start_all" => self.on_button_pressed("Start All Services"),
            "stop_all" => self.on_button_pressed("Stop All Services"),
            "restart_all" => self.on_button_pressed("Restart Services"),
            "view_logs" => self.on_button_pressed("View Logs"),
            "service_details" => self.on_button_pressed("Service Details"),
            "health_checks" => self.on_button_pressed("Run Health Checks"),
            "back" => self.on_button_pressed("Back"),
            "profiles" => self.on_button_pressed("Profiles"),
            "services" => self.on_button_pressed("Services"),
            _ => {
                // Screen-specific button handling
                match self.current_screen.as_str() {
                    "logs" => {
                        if let Some(screen) = &mut self.logs_screen {
                            screen.on_button_pressed(button_id);
                        }
                    },
                    "details" => {
                        if let Some(screen) = &mut self.details_screen {
                            screen.on_button_pressed(button_id);
                        }
                    },
                    "profiles" => {
                        if let Some(screen) = &mut self.profiles_screen {
                            screen.on_button_pressed(button_id);
                        }
                    },
                    "services" => {
                        if let Some(screen) = &mut self.services_screen {
                            screen.on_button_pressed(button_id);
                        }
                    },
                    _ => {}
                }
            }
        }
    }

    pub fn handle_mouse_event(&mut self, x: u16, y: u16, action: MouseAction) {
        // Make a copy of buttons to avoid borrow issues
        let buttons: Vec<(String, Rect)> = self.buttons.clone().into_iter().collect();
        
        // Check buttons
        for (id, rect) in buttons {
            if x >= rect.x && x < rect.x + rect.width && 
               y >= rect.y && y < rect.y + rect.height {
                
                if matches!(action, MouseAction::Clicked) {
                    self.handle_button_click(&id);
                }
                
                self.hover_element = Some(id);
                return;
            }
        }

        // Check clickable areas
        let clickables: Vec<(String, Rect)> = self.clickable_areas.clone().into_iter().collect();
        
        for (id, rect) in clickables {
            if x >= rect.x && x < rect.x + rect.width && 
               y >= rect.y && y < rect.y + rect.height {
                
                // Calculate row index if this is a table
                if id.starts_with("table_") {
                    let row_height = 1; // Adjust based on your table's row height
                    if y >= rect.y {
                        let relative_y = y - rect.y;
                        let row_index = relative_y / row_height;
                        
                        // Handle table row selection
                        match self.current_screen.as_str() {
                            "profiles" => {
                                if let Some(screen) = &mut self.profiles_screen {
                                    if row_index > 0 { // Skip header row
                                        screen.select_row((row_index - 1) as usize);
                                    }
                                }
                            },
                            "services" => {
                                if let Some(screen) = &mut self.services_screen {
                                    if row_index > 0 { // Skip header row
                                        screen.select_row((row_index - 1) as usize);
                                    }
                                }
                            },
                            _ => {}
                        }
                    }
                }
                
                self.hover_element = Some(id);
                return;
            }
        }

        // No element under mouse
        self.hover_element = None;
    }

    pub fn _create_basic_screens(&mut self) {
        self.current_screen = "basic".to_string();
        crankshaft_logs::print_info("Created and installed basic screens");
    }

    fn screen_available(&self, screen_name: &str) -> bool {
        match screen_name {
            "logs" | "details" | "profiles" | "services" => true,
            _ => false,
        }
    }

    pub fn notify(&self, message: &str, title: Option<&str>, severity: Option<&str>) {
        if let Some(t) = title {
            println!("[NOTIFY] {}: {}", t, message);
        } else {
            println!("[NOTIFY] {}", message);
        }
        if let Some(s) = severity {
            println!("Severity: {}", s);
        }
    }

    pub fn render(&mut self) -> io::Result<()> {
        // Clear button and clickable areas before each render
        self.buttons.clear();
        self.clickable_areas.clear();
        
        // Use the renderer to draw the appropriate screen
        match self.current_screen.as_str() {
            "main" => {
                self.terminal.draw(|f| renderer::render_main_screen(
                    f, &self.mock_data, &self.theme, &mut self.buttons, &mut self.clickable_areas, self.hover_element.as_deref()
                ))?;
            },
            "details" => {
                if let Some(screen) = &mut self.details_screen {
                    self.terminal.draw(|f| renderer::render_details_screen(
                        f, screen, &self.mock_data, &self.theme, &mut self.buttons, &mut self.clickable_areas
                    ))?;
                }
            },
            "logs" => {
                if let Some(screen) = &mut self.logs_screen {
                    self.terminal.draw(|f| renderer::render_logs_screen(
                        f, screen, &self.mock_data, &self.theme, &mut self.buttons, &mut self.clickable_areas
                    ))?;
                }
            },
            "profiles" => {
                if let Some(screen) = &mut self.profiles_screen {
                    self.terminal.draw(|f| renderer::render_profiles_screen(
                        f, screen, &self.mock_data, &self.theme, &mut self.buttons, &mut self.clickable_areas
                    ))?;
                }
            },
            "services" => {
                if let Some(screen) = &mut self.services_screen {
                    self.terminal.draw(|f| renderer::render_services_screen(
                        f, screen, &self.mock_data, &self.theme, &mut self.buttons, &mut self.clickable_areas
                    ))?;
                }
            },
            _ => {
                self.terminal.draw(|f| renderer::render_main_screen(
                    f, &self.mock_data, &self.theme, &mut self.buttons, &mut self.clickable_areas, self.hover_element.as_deref()
                ))?;
            }
        }
        
        Ok(())
    }

    pub fn run(&mut self) -> io::Result<()> {
        self.on_mount();
        
        let tick_rate = Duration::from_millis(100);

        loop {
            self.render()?;

            if crossterm::event::poll(tick_rate)? {
                if let event::Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('p') => self.switch_to_profiles(),
                        KeyCode::Char('s') => self.switch_to_services(),
                        KeyCode::Char('l') => self.action_logs(),
                        KeyCode::Char('d') => self.action_details(),
                        KeyCode::Char('r') => {
                            // Handle refresh based on current screen
                            match self.current_screen.as_str() {
                                "main" => self.action_refresh(),
                                "profiles" => {
                                    if let Some(screen) = &mut self.profiles_screen {
                                        screen.action_refresh();
                                    }
                                },
                                "services" => {
                                    if let Some(screen) = &mut self.services_screen {
                                        screen.action_refresh();
                                    }
                                },
                                "logs" => {
                                    if let Some(screen) = &mut self.logs_screen {
                                        screen.action_toggle_follow();
                                    }
                                },
                                "details" => {
                                    if let Some(screen) = &mut self.details_screen {
                                        screen.action_refresh();
                                    }
                                },
                                _ => {}
                            }
                        },
                        KeyCode::Char('c') => {
                            if self.current_screen == "logs" {
                                if let Some(screen) = &mut self.logs_screen {
                                    screen.action_clear_logs();
                                }
                            }
                        },
                        KeyCode::Esc => self.action_back(),
                        KeyCode::Up => {
                            match self.current_screen.as_str() {
                                "profiles" => {
                                    if let Some(screen) = &mut self.profiles_screen {
                                        screen.select_up();
                                    }
                                },
                                "services" => {
                                    if let Some(screen) = &mut self.services_screen {
                                        screen.select_up();
                                    }
                                },
                                _ => {}
                            }
                        },
                        KeyCode::Down => {
                            match self.current_screen.as_str() {
                                "profiles" => {
                                    if let Some(screen) = &mut self.profiles_screen {
                                        screen.select_down();
                                    }
                                },
                                "services" => {
                                    if let Some(screen) = &mut self.services_screen {
                                        screen.select_down();
                                    }
                                },
                                _ => {}
                            }
                        },
                        KeyCode::Char('1') => {
                            match self.current_screen.as_str() {
                                "services" => {
                                    if let Some(screen) = &mut self.services_screen {
                                        screen.on_button_pressed("start");
                                    }
                                },
                                _ => self.on_button_pressed("Start All Services"),
                            }
                        },
                        KeyCode::Char('2') => {
                            match self.current_screen.as_str() {
                                "services" => {
                                    if let Some(screen) = &mut self.services_screen {
                                        screen.on_button_pressed("stop");
                                    }
                                },
                                _ => self.on_button_pressed("Stop All Services"),
                            }
                        },
                        KeyCode::Char('3') => {
                            match self.current_screen.as_str() {
                                "services" => {
                                    if let Some(screen) = &mut self.services_screen {
                                        screen.on_button_pressed("restart");
                                    }
                                },
                                _ => self.on_button_pressed("Restart Services"),
                            }
                        },
                        KeyCode::Char('4') => self.on_button_pressed("View Logs"),
                        KeyCode::Char('5') => self.on_button_pressed("Service Details"),
                        KeyCode::Char('6') => self.on_button_pressed("Run Health Checks"),
                        _ => {}
                    }
                } else if let event::Event::Mouse(mouse) = event::read()? {
                    let action = match mouse.kind {
                        event::MouseEventKind::Down(_) => MouseAction::Pressed,
                        event::MouseEventKind::Up(_) => {
                            // Handle click when mouse is released
                            self.handle_mouse_event(mouse.column, mouse.row, MouseAction::Clicked);
                            MouseAction::Released
                        },
                        event::MouseEventKind::Drag(_) => MouseAction::Moved,
                        event::MouseEventKind::ScrollDown => MouseAction::ScrollDown,
                        event::MouseEventKind::ScrollUp => MouseAction::ScrollUp,
                        _ => MouseAction::Moved,
                    };
                    
                    self.handle_mouse_event(mouse.column, mouse.row, action);
                }
            }

            // Update real-time components
            if self.current_screen == "logs" {
                if let Some(screen) = &mut self.logs_screen {
                    if screen.following {
                        screen.add_new_log_entry();
                    }
                }
            }
        }

        Ok(())
    }
}

pub fn run() -> io::Result<()> {
    crankshaft_logs::print_info("Inside run() function");
    let mut app = CrankshaftTUI::new()?;
    
    let result = app.run();
    
    // Properly clean up terminal
    disable_raw_mode()?;
    execute!(
        app.terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    app.terminal.show_cursor()?;
    
    result
}

pub fn main() -> io::Result<()> {
    crankshaft_logs::print_info("Starting Crankshaft TUI...");
    let result = run();
    if let Err(ref err) = result {
        eprintln!("Error: {}", err);
    }
    result
}
