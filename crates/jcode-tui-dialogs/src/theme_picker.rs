use ratatui::{
    layout::Rect,
    style::{Color, Style, Stylize},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::dialog::{DialogAction, DialogOverlay};

pub struct ThemePicker {
    themes: Vec<String>,
    selected: usize,
    title: String,
}

impl ThemePicker {
    pub fn new(themes: Vec<String>) -> Self {
        Self {
            themes,
            selected: 0,
            title: "Select Theme".to_string(),
        }
    }

    pub fn selected_theme(&self) -> Option<&str> {
        self.themes.get(self.selected).map(|s| s.as_str())
    }
}

impl DialogOverlay for ThemePicker {
    fn title(&self) -> &str {
        &self.title
    }

    fn height(&self) -> u16 {
        (self.themes.len() as u16 + 2).min(20)
    }

    fn width(&self) -> u16 {
        40
    }

    fn render(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(format!(" {} ", self.title))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Magenta));

        let items: Vec<ListItem> = self
            .themes
            .iter()
            .enumerate()
            .map(|(i, t)| {
                if i == self.selected {
                    ListItem::new(t.clone()).style(Style::default().fg(Color::Black).bg(Color::Magenta))
                } else {
                    ListItem::new(t.clone())
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
                if self.selected < self.themes.len().saturating_sub(1) {
                    self.selected += 1;
                }
                DialogAction::None
            }
            crossterm::event::KeyCode::Enter => {
                DialogAction::Select(self.themes[self.selected].clone())
            }
            crossterm::event::KeyCode::Esc => DialogAction::Cancel,
            _ => DialogAction::None,
        }
    }
}
