use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::Text,
    widgets::{Block, Borders, BorderType, Paragraph, Widget},
};

#[derive(Debug, Clone)]
pub struct Button<'a> {
    content: Text<'a>,
    style: Style,
    hover_style: Style,
    rect: Rect,
    is_hovered: bool,
    is_pressed: bool,
    id: String,
}

impl<'a> Button<'a> {
    pub fn new<T>(content: T) -> Self
    where
        T: Into<Text<'a>>,
    {
        Self {
            content: content.into(),
            style: Style::default().bg(Color::DarkGray),
            hover_style: Style::default().bg(Color::Blue),
            rect: Rect::default(),
            is_hovered: false,
            is_pressed: false,
            id: String::new(),
        }
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn hover_style(mut self, style: Style) -> Self {
        self.hover_style = style;
        self
    }

    pub fn is_inside(&self, x: u16, y: u16) -> bool {
        x >= self.rect.x && x < self.rect.x + self.rect.width &&
        y >= self.rect.y && y < self.rect.y + self.rect.height
    }

    pub fn set_hover(&mut self, is_hover: bool) {
        self.is_hovered = is_hover;
    }

    pub fn set_pressed(&mut self, is_pressed: bool) {
        self.is_pressed = is_pressed;
    }

    pub fn get_id(&self) -> &str {
        &self.id
    }
}

impl<'a> Widget for Button<'a> {
    fn render(mut self, area: Rect, buf: &mut Buffer) {
        self.rect = area;
        let current_style = if self.is_pressed {
            self.hover_style.add_modifier(ratatui::style::Modifier::REVERSED)
        } else if self.is_hovered {
            self.hover_style
        } else {
            self.style
        };

        let paragraph = Paragraph::new(self.content)
            .style(current_style)
            .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded));
        
        paragraph.render(area, buf);
    }
}

// ClickableList widget for making tables and lists clickable
#[derive(Debug, Clone)]
pub struct ClickableList {
    rect: Rect,
    selected_index: Option<usize>,
    is_mouse_over: bool,
}

impl ClickableList {
    pub fn new() -> Self {
        Self {
            rect: Rect::default(),
            selected_index: None,
            is_mouse_over: false,
        }
    }

    pub fn set_rect(&mut self, rect: Rect) {
        self.rect = rect;
    }

    pub fn handle_mouse_event(&mut self, x: u16, y: u16, row_height: u16) -> Option<usize> {
        if x >= self.rect.x && x < self.rect.x + self.rect.width &&
           y >= self.rect.y && y < self.rect.y + self.rect.height {
            self.is_mouse_over = true;
            
            // Calculate row index based on y position
            if y >= self.rect.y {
                let relative_y = y - self.rect.y;
                let index = relative_y / row_height;
                let max_rows = self.rect.height / row_height;
                
                if index < max_rows {
                    self.selected_index = Some(index as usize);
                    return self.selected_index;
                }
            }
        } else {
            self.is_mouse_over = false;
        }
        None
    }
}
