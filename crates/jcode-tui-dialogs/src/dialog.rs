use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Style, Stylize},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub trait DialogOverlay {
    fn title(&self) -> &str;
    fn height(&self) -> u16;
    fn width(&self) -> u16;
    fn render(&self, f: &mut Frame, area: Rect);
    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> DialogAction;

    fn centered_rect(&self, max: Rect) -> Rect {
        let width = self.width().min(max.width.saturating_sub(4));
        let height = self.height().min(max.height.saturating_sub(4));
        let x = (max.width - width) / 2;
        let y = (max.height - height) / 2;
        Rect { x, y, width, height }
    }

    fn render_background(&self, f: &mut Frame) {
        let area = f.area();
        f.render_widget(Clear, area);
        let block = Block::default()
            .style(Style::default().bg(Color::DarkGray).fg(Color::White));
        f.render_widget(block, area);
    }
}

pub enum DialogAction {
    Close,
    Confirm,
    Cancel,
    Select(String),
    None,
    Changed,
}
