use crate::presentation::tui::app::App;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

pub fn render_lobby(f: &mut Frame, area: Rect, app: &App) {
    let lobby_tab = &app.lobby_tab;

    let text = if let Some(lobby_name) = lobby_tab.lobby_name() {
        vec![
            Line::from(vec![
                Span::styled("Lobby: ", Style::default().fg(Color::Cyan)),
                Span::raw(lobby_name),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Participants: ", Style::default().fg(Color::Cyan)),
                Span::raw(lobby_tab.participant_count().to_string()),
            ]),
        ]
    } else {
        vec![
            Line::from("Not in a lobby"),
            Line::from(""),
            Line::from("Waiting for connection..."),
        ]
    };

    let paragraph =
        Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("Lobby Info"));

    f.render_widget(paragraph, area);
}
