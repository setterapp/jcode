use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Style, Stylize},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::dialog::{DialogAction, DialogOverlay};

pub struct ConfirmDialog {
    message: String,
    title: String,
    confirmed: bool,
}

impl ConfirmDialog {
    pub fn new(message: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            title: title.into(),
            confirmed: true,
        }
    }

    pub fn is_confirmed(&self) -> bool {
        self.confirmed
    }
}

impl DialogOverlay for ConfirmDialog {
    fn title(&self) -> &str {
        &self.title
    }

    fn height(&self) -> u16 {
        5
    }

    fn width(&self) -> u16 {
        60
    }

    fn render(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(format!(" {} ", self.title))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red));

        let text = Paragraph::new(self.message.clone())
            .block(block)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        f.render_widget(text, area);
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> DialogAction {
        match key.code {
            crossterm::event::KeyCode::Char('y') | crossterm::event::KeyCode::Enter => {
                self.confirmed = true;
                DialogAction::Confirm
            }
            crossterm::event::KeyCode::Char('n') | crossterm::event::KeyCode::Esc => {
                self.confirmed = false;
                DialogAction::Cancel
            }
            _ => DialogAction::None,
        }
    }
}
