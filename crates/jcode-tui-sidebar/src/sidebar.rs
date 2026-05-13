use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
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
    /// Current session name shown at top (empty = no session selected)
    session_label: String,
    /// Current model name shown at top
    model_label: String,
    /// Whether daemon is connected
    daemon_connected: bool,
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
            width: 26,
            session_label: String::new(),
            model_label: String::new(),
            daemon_connected: false,
        }
    }

    pub fn set_session(&mut self, label: &str) {
        self.session_label = if label.is_empty() { String::new() } else { label.to_string() };
    }

    pub fn set_model(&mut self, label: &str) {
        self.model_label = if label.is_empty() { String::new() } else { label.to_string() };
    }

    pub fn set_daemon_connected(&mut self, connected: bool) {
        self.daemon_connected = connected;
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

        // Split into header info + navigation items
        let has_status = !self.session_label.is_empty() || !self.model_label.is_empty();
        let header_height = if has_status { 3u16 } else { 1u16 };
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(header_height),
                Constraint::Min(1),
            ])
            .split(area);

        let block = Block::default()
            .title(" jcode ")
            .borders(Borders::RIGHT)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(Color::Rgb(16, 18, 26)));

        // ── Status header ──
        if has_status {
            let mut status_lines = Vec::new();
            if !self.model_label.is_empty() {
                let model_display = if self.model_label.len() > 18 {
                    format!("{}…", &self.model_label[..18])
                } else {
                    self.model_label.clone()
                };
                status_lines.push(
                    ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled("◆ ", Style::default().fg(Color::Cyan)),
                        ratatui::text::Span::styled(model_display, Style::default().fg(Color::White)),
                    ])
                );
            }
            if !self.session_label.is_empty() {
                let sess_display = if self.session_label.len() > 18 {
                    format!("{}…", &self.session_label[..18])
                } else {
                    self.session_label.clone()
                };
                status_lines.push(
                    ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled("◈ ", Style::default().fg(Color::Green)),
                        ratatui::text::Span::styled(sess_display, Style::default().fg(Color::Rgb(200, 200, 200))),
                    ])
                );
            }
            // Daemon status
            let daemon_text = if self.daemon_connected { "● daemon" } else { "○ daemon" };
            status_lines.push(
                ratatui::text::Line::from(vec![
                    ratatui::text::Span::styled(
                        daemon_text,
                        Style::default().fg(if self.daemon_connected { Color::Green } else { Color::DarkGray }),
                    ),
                ])
            );
            let status_para = Paragraph::new(ratatui::text::Text::from(status_lines))
                .style(Style::default().bg(Color::Rgb(16, 18, 26)));
            f.render_widget(status_para, chunks[0]);
            f.render_widget(
                Block::default()
                    .style(Style::default().bg(Color::Rgb(24, 28, 38)))
                    .borders(Borders::BOTTOM)
                    .border_style(Style::default().fg(Color::Rgb(48, 54, 61))),
                chunks[0],
            );
        }

        // ── Navigation items ──
        let nav_area = Rect {
            x: area.x,
            y: chunks[1].y,
            width: self.width,
            height: chunks[1].height,
        };

        let items: Vec<ListItem> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let display = format!(" {}  {}", item.icon, item.label);
                if i == self.selected {
                    ListItem::new(display)
                        .style(Style::default().fg(Color::Black).bg(Color::Cyan))
                } else {
                    ListItem::new(display)
                        .style(Style::default().fg(Color::Rgb(200, 200, 220)))
                }
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_symbol("▸")
            .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
        f.render_widget(list, nav_area);
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
