use ratatui::{
    layout::Rect,
    style::{Color, Style, Stylize},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::dialog::{DialogAction, DialogOverlay};

pub struct SessionListDialog {
    sessions: Vec<(String, String)>,
    selected: usize,
    title: String,
}

impl SessionListDialog {
    pub fn new(sessions: Vec<(String, String)>, title: impl Into<String>) -> Self {
        Self {
            sessions,
            selected: 0,
            title: title.into(),
        }
    }

    pub fn selected_session(&self) -> Option<&str> {
        self.sessions.get(self.selected).map(|(id, _)| id.as_str())
    }
}

impl DialogOverlay for SessionListDialog {
    fn title(&self) -> &str {
        &self.title
    }

    fn height(&self) -> u16 {
        (self.sessions.len() as u16 + 2).min(24)
    }

    fn width(&self) -> u16 {
        60
    }

    fn render(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(format!(" {} ", self.title))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        let items: Vec<ListItem> = self
            .sessions
            .iter()
            .enumerate()
            .map(|(i, (id, name))| {
                let display = format!("{}  {}", id, name);
                if i == self.selected {
                    ListItem::new(display).style(Style::default().fg(Color::Black).bg(Color::Yellow))
                } else {
                    ListItem::new(display)
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
                if self.selected < self.sessions.len().saturating_sub(1) {
                    self.selected += 1;
                }
                DialogAction::None
            }
            crossterm::event::KeyCode::Enter => {
                DialogAction::Select(self.sessions[self.selected].0.clone())
            }
            crossterm::event::KeyCode::Esc => DialogAction::Cancel,
            _ => DialogAction::None,
        }
    }
}
