use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph, Clear},
    Frame,
};

#[derive(Clone, Debug)]
pub enum PaletteAction {
    // Navigation
    OpenSessions,
    OpenModels,
    OpenAgents,
    OpenMcp,
    OpenSettings,
    ToggleSidebar,
    Quit,
    // Session actions
    RenameSession,
    ForkSession,
    StashSession,
    ClearChat,
    ToggleDiagram,
    // Dynamic selections
    SelectModel(String),
    SelectSession(String),
}

pub struct CommandEntry {
    pub name: String,
    pub description: String,
    pub action: PaletteAction,
    /// Category for grouping in the UI
    pub category: &'static str,
}

pub struct CommandPalette {
    commands: Vec<CommandEntry>,
    filtered: Vec<usize>,
    selected: usize,
    query: String,
    visible: bool,
    /// Dynamic model entries (loaded from provider)
    models: Vec<String>,
    /// Dynamic session entries (loaded from storage)
    sessions: Vec<(String, String)>, // (id, title or truncated id)
}

impl CommandPalette {
    pub fn new() -> Self {
        let commands = vec![
            // Session & navigation
            CommandEntry { name: "Sessions".into(), description: "List and switch sessions".into(), action: PaletteAction::OpenSessions, category: "Navigation" },
            CommandEntry { name: "Models".into(), description: "Switch AI model".into(), action: PaletteAction::OpenModels, category: "Navigation" },
            CommandEntry { name: "Sidebar".into(), description: "Toggle navigation sidebar".into(), action: PaletteAction::ToggleSidebar, category: "Navigation" },
            CommandEntry { name: "Quit".into(), description: "Exit".into(), action: PaletteAction::Quit, category: "Navigation" },
            // Session management
            CommandEntry { name: "Fork Session".into(), description: "Fork from this point".into(), action: PaletteAction::ForkSession, category: "Session" },
            CommandEntry { name: "Rename Session".into(), description: "Rename current session".into(), action: PaletteAction::RenameSession, category: "Session" },
            CommandEntry { name: "Stash".into(), description: "Stash current state".into(), action: PaletteAction::StashSession, category: "Session" },
            CommandEntry { name: "Clear Chat".into(), description: "Clear chat messages".into(), action: PaletteAction::ClearChat, category: "Session" },
            // Tools
            CommandEntry { name: "MCP".into(), description: "Manage MCP servers".into(), action: PaletteAction::OpenMcp, category: "Tools" },
            CommandEntry { name: "Agents".into(), description: "Select agent mode".into(), action: PaletteAction::OpenAgents, category: "Tools" },
            CommandEntry { name: "Settings".into(), description: "Open settings".into(), action: PaletteAction::OpenSettings, category: "Tools" },
            CommandEntry { name: "Diagram".into(), description: "Toggle diagram pane".into(), action: PaletteAction::ToggleDiagram, category: "Tools" },
        ];
        let filtered: Vec<usize> = (0..commands.len()).collect();
        Self { commands, filtered, selected: 0, query: String::new(), visible: false, models: Vec::new(), sessions: Vec::new() }
    }

    /// Set available models for model-switching commands
    pub fn set_models(&mut self, models: Vec<String>) {
        self.models = models.clone();
        // Remove old SelectModel commands and add fresh ones
        self.commands.retain(|c| !matches!(c.action, PaletteAction::SelectModel(_)));
        for model in &models {
            let name = model.clone();
            self.commands.push(CommandEntry {
                name: format!("Set Model: {}", model),
                description: format!("Switch to {}", model),
                action: PaletteAction::SelectModel(name),
                category: "Model",
            });
        }
        self.filter();
    }

    /// Set available sessions for session-switching commands
    pub fn set_sessions(&mut self, sessions: Vec<(String, String)>) {
        self.sessions = sessions;
        self.commands.retain(|c| !matches!(c.action, PaletteAction::SelectSession(_)));
        for (id, title) in &self.sessions {
            let display = if title.is_empty() { format!("session {}", &id[..id.len().min(12)]) } else { title.clone() };
            self.commands.push(CommandEntry {
                name: format!("Session: {}", display),
                description: id.clone(),
                action: PaletteAction::SelectSession(id.clone()),
                category: "Session",
            });
        }
        self.filter();
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        if self.visible {
            self.query.clear();
            self.selected = 0;
            self.filter();
        }
    }

    pub fn show(&mut self) {
        if !self.visible {
            self.toggle();
        }
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn visible(&self) -> bool {
        self.visible
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        let width = 60.min(area.width.saturating_sub(8));
        let height = (self.filtered.len() as u16 + 4).min(area.height.saturating_sub(8));
        let x = (area.width - width) / 2;
        let y = (area.height - height) / 2;
        let dialog_area = Rect { x, y, width, height };

        f.render_widget(Clear, dialog_area);

        let block = Block::default()
            .title(" Command Palette ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(Color::Rgb(30, 30, 40)));

        let inner = block.inner(dialog_area);

        // Query input line
        let query_display = if self.query.is_empty() {
            " Type to search..."
        } else {
            &self.query
        };
        let query_para = Paragraph::new(query_display)
            .style(Style::default().fg(Color::White));
        f.render_widget(query_para, Rect { x: inner.x, y: inner.y, width: inner.width, height: 1 });

        // Category header and results
        let items: Vec<ListItem> = self
            .filtered
            .iter()
            .enumerate()
            .map(|(i, &idx)| {
                let cmd = &self.commands[idx];
                let display = format!(" {}  {}", cmd.name, cmd.description);
                if i == self.selected {
                    ListItem::new(format!("{} [{}]", display, cmd.category))
                        .style(Style::default().fg(Color::Black).bg(Color::Cyan))
                } else {
                    ListItem::new(display)
                        .style(Style::default().fg(match cmd.category {
                            "Model" => Color::Yellow,
                            "Session" => Color::Green,
                            "Navigation" => Color::White,
                            "Tools" => Color::Rgb(180, 180, 255),
                            _ => Color::Gray,
                        }))
                }
            })
            .collect();

        let list = List::new(items)
            .highlight_symbol("> ")
            .block(Block::default());

        // Count info
        let info = format!(" {} results ({} total)", self.filtered.len(), self.commands.len());
        let info_para = Paragraph::new(info)
            .style(Style::default().fg(Color::DarkGray));
        let info_y = inner.y + inner.height.saturating_sub(1);

        let list_area = Rect {
            x: inner.x,
            y: inner.y + 1,
            width: inner.width,
            height: inner.height.saturating_sub(2).max(1),
        };
        f.render_widget(list, list_area);
        f.render_widget(info_para, Rect { x: inner.x, y: info_y, width: inner.width, height: 1 });
        f.render_widget(block, dialog_area);
    }

    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> Option<PaletteAction> {
        match key.code {
            crossterm::event::KeyCode::Esc => {
                self.visible = false;
                None
            }
            crossterm::event::KeyCode::Enter => {
                if !self.filtered.is_empty() {
                    let action = self.commands[self.filtered[self.selected]].action.clone();
                    self.visible = false;
                    Some(action)
                } else {
                    None
                }
            }
            crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Char('k') => {
                if self.selected > 0 { self.selected -= 1; }
                None
            }
            crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Char('j') => {
                if self.selected < self.filtered.len().saturating_sub(1) { self.selected += 1; }
                None
            }
            crossterm::event::KeyCode::Backspace => {
                self.query.pop();
                self.selected = 0;
                self.filter();
                None
            }
            crossterm::event::KeyCode::Char(c) => {
                self.query.push(c);
                self.selected = 0;
                self.filter();
                None
            }
            _ => None,
        }
    }

    fn filter(&mut self) {
        let q = self.query.to_lowercase();
        if q.is_empty() {
            self.filtered = (0..self.commands.len()).collect();
        } else {
            self.filtered = self.commands
                .iter()
                .enumerate()
                .filter(|(_, cmd)| {
                    cmd.name.to_lowercase().contains(&q)
                        || cmd.description.to_lowercase().contains(&q)
                        || cmd.category.to_lowercase().contains(&q)
                })
                .map(|(i, _)| i)
                .collect();
        }
        if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len().saturating_sub(1);
        }
    }
}

impl Default for CommandPalette {
    fn default() -> Self {
        Self::new()
    }
}
