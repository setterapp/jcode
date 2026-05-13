use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Style, Stylize},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::dialog::{DialogAction, DialogOverlay};

pub struct ModelPicker {
    models: Vec<String>,
    selected: usize,
    title: String,
}

impl ModelPicker {
    pub fn new(models: Vec<String>) -> Self {
        Self {
            models,
            selected: 0,
            title: "Select Model".to_string(),
        }
    }

    pub fn selected_model(&self) -> Option<&str> {
        self.models.get(self.selected).map(|s| s.as_str())
    }
}

impl DialogOverlay for ModelPicker {
    fn title(&self) -> &str {
        &self.title
    }

    fn height(&self) -> u16 {
        (self.models.len() as u16 + 2).min(20)
    }

    fn width(&self) -> u16 {
        50
    }

    fn render(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(format!(" {} ", self.title))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let inner = block.inner(area);

        let items: Vec<ListItem> = self
            .models
            .iter()
            .enumerate()
            .map(|(i, m)| {
                if i == self.selected {
                    ListItem::new(m.clone()).style(Style::default().fg(Color::Black).bg(Color::Cyan))
                } else {
                    ListItem::new(m.clone())
                }
            })
            .collect();

        let list = List::new(items).block(block).highlight_symbol("> ");
        f.render_widget(list, area);
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> DialogAction {
        match key.code {
            crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Char('k') => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
                DialogAction::None
            }
            crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Char('j') => {
                if self.selected < self.models.len().saturating_sub(1) {
                    self.selected += 1;
                }
                DialogAction::None
            }
            crossterm::event::KeyCode::Enter => {
                DialogAction::Select(self.models[self.selected].clone())
            }
            crossterm::event::KeyCode::Esc => DialogAction::Cancel,
            _ => DialogAction::None,
        }
    }
}
