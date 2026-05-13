use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub struct SidebarItem {
    pub label: String,
    pub icon: String,
    pub action: SidebarAction,
}

pub enum SidebarAction {
    OpenSessions,
    OpenSettings,
    OpenModels,
    OpenAgents,
    OpenMcp,
    OpenHelp,
    Quit,
}

pub struct Sidebar {
    items: Vec<SidebarItem>,
    selected: usize,
    visible: bool,
    width: u16,
}

impl Sidebar {
    pub fn new() -> Self {
        Self {
            items: vec![
                SidebarItem { label: "Sessions".into(), icon: "💬".into(), action: SidebarAction::OpenSessions },
                SidebarItem { label: "Models".into(), icon: "🧠".into(), action: SidebarAction::OpenModels },
                SidebarItem { label: "Agents".into(), icon: "🤖".into(), action: SidebarAction::OpenAgents },
                SidebarItem { label: "MCP".into(), icon: "🔌".into(), action: SidebarAction::OpenMcp },
                SidebarItem { label: "Settings".into(), icon: "⚙".into(), action: SidebarAction::OpenSettings },
                SidebarItem { label: "Help".into(), icon: "?".into(), action: SidebarAction::OpenHelp },
            ],
            selected: 0,
            visible: false,
            width: 24,
        }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn visible(&self) -> bool {
        self.visible
    }

    pub fn width(&self) -> u16 {
        if self.visible { self.width } else { 0 }
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        let block = Block::default()
            .title(" Navigation ")
            .borders(Borders::RIGHT)
            .border_style(Style::default().fg(Color::DarkGray))
            .style(Style::default().bg(Color::Rgb(20, 20, 30)));

        let items: Vec<ListItem> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let display = format!(" {}  {}", item.icon, item.label);
                if i == self.selected {
                    ListItem::new(display).style(Style::default().fg(Color::Black).bg(Color::Cyan))
                } else {
                    ListItem::new(display)
                }
            })
            .collect();

        let list = List::new(items).block(block).highlight_symbol("▸");
        f.render_widget(list, area);
    }

    pub fn next(&mut self) {
        if self.selected < self.items.len().saturating_sub(1) {
            self.selected += 1;
        }
    }

    pub fn prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn select_current(&self) -> &SidebarAction {
        &self.items[self.selected].action
    }
}
