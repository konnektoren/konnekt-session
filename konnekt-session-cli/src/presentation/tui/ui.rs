use super::app::{App, Tab};
use konnekt_session_core::ParticipationMode;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs},
};

pub fn render(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Footer
        ])
        .split(f.area());

    render_header(f, chunks[0], app);
    render_content(f, chunks[1], app);
    render_footer(f, chunks[2]);
}

fn render_header(f: &mut Frame, area: Rect, app: &App) {
    let titles = vec![
        Tab::Session.title(),
        Tab::Lobby.title(),
        Tab::Events.title(),
        Tab::Participants.title(),
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

fn render_content(f: &mut Frame, area: Rect, app: &App) {
    match app.current_tab {
        Tab::Session => render_session(f, area, app),
        Tab::Lobby => render_lobby(f, area, app),
        Tab::Events => render_events(f, area, app),
        Tab::Participants => render_participants(f, area, app),
        Tab::Help => render_help(f, area),
    }
}

fn render_session(f: &mut Frame, area: Rect, app: &App) {
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
            &app.session_id,
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
            format!("konnekt-tui join --session-id {}", app.session_id),
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
    if let Some(msg) = &app.clipboard_message {
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

    if let Some(peer_id) = &app.local_peer_id {
        text.push(Line::from(vec![
            Span::styled("Local Peer ID: ", Style::default().fg(Color::Cyan)),
            Span::raw(peer_id),
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

fn render_lobby(f: &mut Frame, area: Rect, app: &App) {
    let text = if let Some(lobby) = app.session_state.lobby() {
        vec![
            Line::from(vec![
                Span::styled("Lobby: ", Style::default().fg(Color::Cyan)),
                Span::raw(lobby.name()),
            ]),
            Line::from(vec![
                Span::styled("ID: ", Style::default().fg(Color::Cyan)),
                Span::raw(lobby.id().to_string()),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Participants: ", Style::default().fg(Color::Cyan)),
                Span::raw(lobby.participants().len().to_string()),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Your Role: ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    format!("{}", app.session_state.participant().lobby_role()),
                    if app.session_state.is_host() {
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    },
                ),
            ]),
            Line::from(vec![
                Span::styled("Your Mode: ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    format!("{}", app.session_state.participant().participation_mode()),
                    if app.session_state.participant().can_submit_results() {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::Yellow)
                    },
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::raw("Press "),
                Span::styled(
                    "t",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" to toggle between Active/Spectating"),
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

fn render_events(f: &mut Frame, area: Rect, app: &App) {
    let events: Vec<ListItem> = app
        .event_log
        .iter()
        .skip(app.scroll_offset)
        .map(|e| ListItem::new(e.as_str()))
        .collect();

    let list = List::new(events)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Event Log ({})", app.event_log.len())),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(list, area);
}

fn render_participants(f: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = if let Some(lobby) = app.session_state.lobby() {
        lobby
            .participants()
            .values()
            .enumerate()
            .map(|(idx, p)| {
                let role_icon = if p.is_host() { "👑" } else { "👤" };

                // Color based on participation mode
                let (mode_text, mode_style) = match p.participation_mode() {
                    konnekt_session_core::ParticipationMode::Active => {
                        ("🎮 Active", Style::default().fg(Color::Green))
                    }
                    konnekt_session_core::ParticipationMode::Spectating => {
                        ("👁️  Spectating", Style::default().fg(Color::Yellow))
                    }
                };

                // Highlight selected participant (if host)
                let selected = app.session_state.is_host()
                    && app.current_tab == Tab::Participants
                    && idx == app.selected_participant;

                let prefix = if selected { "> " } else { "  " };

                // Format: "> 👑 Alice - 🎮 Active"
                let text = vec![
                    Span::raw(prefix),
                    Span::raw(format!("{} ", role_icon)),
                    Span::styled(
                        p.name(),
                        if p.is_host() {
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Color::White)
                        },
                    ),
                    Span::raw(" - "),
                    Span::styled(mode_text, mode_style),
                ];

                let mut item = ListItem::new(Line::from(text));

                if selected {
                    item = item.style(Style::default().bg(Color::DarkGray));
                }

                item
            })
            .collect()
    } else {
        vec![ListItem::new("No participants")]
    };

    let title = if app.session_state.is_host() {
        "Participants (j/k: select, x: kick)" // Changed from 'k: kick'
    } else {
        "Participants"
    };

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(title));

    f.render_widget(list, area);
}

fn render_help(f: &mut Frame, area: Rect) {
    let text = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "Session Tab:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  y", Style::default().fg(Color::Yellow)),
            Span::raw("  Copy Session ID to clipboard"),
        ]),
        Line::from(vec![
            Span::styled("  c", Style::default().fg(Color::Yellow)),
            Span::raw("  Copy join command to clipboard"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Participants Tab:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  t", Style::default().fg(Color::Yellow)),
            Span::raw("  Toggle Active ↔ Spectating mode"),
        ]),
        Line::from(vec![
            Span::styled("  j/k", Style::default().fg(Color::Yellow)),
            Span::raw("  Navigate participants (host only)"),
        ]),
        Line::from(vec![
            Span::styled("  x", Style::default().fg(Color::Yellow)),
            Span::raw("  Kick selected guest (host only)"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Navigation:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  Tab / →", Style::default().fg(Color::Yellow)),
            Span::raw("  Next tab"),
        ]),
        Line::from(vec![
            Span::styled("  Shift+Tab / ←", Style::default().fg(Color::Yellow)),
            Span::raw("  Previous tab"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  q / Esc", Style::default().fg(Color::Yellow)),
            Span::raw("  Quit"),
        ]),
    ];

    let paragraph = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Keyboard Shortcuts"),
    );

    f.render_widget(paragraph, area);
}

fn render_footer(f: &mut Frame, area: Rect) {
    let text = Line::from(vec![
        Span::styled("y", Style::default().fg(Color::Green)),
        Span::raw(" copy ID | "),
        Span::styled("c", Style::default().fg(Color::Green)),
        Span::raw(" copy cmd | "),
        Span::styled("t", Style::default().fg(Color::Yellow)),
        Span::raw(" toggle mode | "),
        Span::styled("x", Style::default().fg(Color::Yellow)),
        Span::raw(" kick | "),
        Span::styled("Tab", Style::default().fg(Color::Yellow)),
        Span::raw(" switch | "),
        Span::styled("q", Style::default().fg(Color::Yellow)),
        Span::raw(" quit"),
    ]);

    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Gray));

    f.render_widget(paragraph, area);
}
