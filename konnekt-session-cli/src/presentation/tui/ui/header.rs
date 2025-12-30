use crate::presentation::tui::app::{App, Tab};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Tabs},
};

pub fn render_header(f: &mut Frame, area: Rect, app: &App) {
    let titles = vec![
        Tab::Session.title(),
        Tab::Lobby.title(),
        Tab::Activities.title(),
        Tab::Participants.title(),
        Tab::Results.title(),
        Tab::Events.title(),
        Tab::Help.title(),
    ];

    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title("Konnekt TUI"))
        .select(app.current_tab as usize)
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(tabs, area);
}
