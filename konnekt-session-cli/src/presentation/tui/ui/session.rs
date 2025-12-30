use crate::presentation::tui::app::App;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

pub fn render_session(f: &mut Frame, area: Rect, app: &App) {
    let session_tab = &app.session_tab;

    let mut text = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "Session ID:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            session_tab.session_id(),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::raw("Press "),
            Span::styled(
                "y",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" to copy Session ID to clipboard"),
        ]),
        Line::from(""),
        Line::from("─".repeat(50)),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Share Command:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            format!("konnekt-tui join --session-id {}", session_tab.session_id()),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::raw("Press "),
            Span::styled(
                "c",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" to copy join command to clipboard"),
        ]),
    ];

    // Show clipboard message if active
    if let Some(msg) = session_tab.clipboard_message() {
        text.push(Line::from(""));
        text.push(Line::from(""));
        text.push(Line::from(vec![Span::styled(
            msg,
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]));
    }

    text.push(Line::from(""));
    text.push(Line::from(""));

    // Connection status
    if let Some(peer_id) = session_tab.local_peer_id() {
        text.push(Line::from(vec![
            Span::styled("Local Peer ID: ", Style::default().fg(Color::Cyan)),
            Span::raw(peer_id),
        ]));
        text.push(Line::from(vec![
            Span::styled("Connected Peers: ", Style::default().fg(Color::Cyan)),
            Span::raw(session_tab.peer_count().to_string()),
        ]));
    } else {
        text.push(Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::Cyan)),
            Span::raw("Connecting..."),
        ]));
    }

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Session Information"),
        )
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);
}
