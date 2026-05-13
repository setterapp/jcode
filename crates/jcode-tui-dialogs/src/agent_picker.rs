use ratatui::{
    layout::Rect,
    style::{Color, Style, Stylize},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::dialog::{DialogAction, DialogOverlay};

pub struct AgentPicker {
    agents: Vec<String>,
    selected: usize,
    title: String,
}

impl AgentPicker {
    pub fn new(agents: Vec<String>) -> Self {
        Self {
            agents,
            selected: 0,
            title: "Select Agent".to_string(),
        }
    }

    pub fn selected_agent(&self) -> Option<&str> {
        self.agents.get(self.selected).map(|s| s.as_str())
    }
}

impl DialogOverlay for AgentPicker {
    fn title(&self) -> &str {
        &self.title
    }

    fn height(&self) -> u16 {
        (self.agents.len() as u16 + 2).min(20)
    }

    fn width(&self) -> u16 {
        50
    }

    fn render(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(format!(" {} ", self.title))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green));

        let items: Vec<ListItem> = self
            .agents
            .iter()
            .enumerate()
            .map(|(i, a)| {
                if i == self.selected {
                    ListItem::new(a.clone()).style(Style::default().fg(Color::Black).bg(Color::Green))
                } else {
                    ListItem::new(a.clone())
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
                if self.selected < self.agents.len().saturating_sub(1) {
                    self.selected += 1;
                }
                DialogAction::None
            }
            crossterm::event::KeyCode::Enter => {
                DialogAction::Select(self.agents[self.selected].clone())
            }
            crossterm::event::KeyCode::Esc => DialogAction::Cancel,
            _ => DialogAction::None,
        }
    }
}
