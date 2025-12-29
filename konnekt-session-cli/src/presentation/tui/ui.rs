use super::app::{App, Tab};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
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
    let text = vec![
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
            Span::styled("💡 ", Style::default().fg(Color::Yellow)),
            Span::raw("You can select and copy this ID from your terminal"),
        ]),
        Line::from(""),
        Line::from("Share this command with guests:"),
        Line::from(""),
        Line::from(vec![Span::styled(
            format!("konnekt-tui join --session-id {}", app.session_id),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(""),
        if let Some(peer_id) = &app.local_peer_id {
            Line::from(vec![
                Span::styled("Local Peer ID: ", Style::default().fg(Color::Cyan)),
                Span::raw(peer_id),
            ])
        } else {
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::Cyan)),
                Span::raw("Connecting..."),
            ])
        },
    ];

    let paragraph = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Session Information"),
    );

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
                Span::raw(format!(
                    "{}",
                    app.session_state.participant().participation_mode()
                )),
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
            .map(|p| {
                let role_icon = if p.is_host() { "👑" } else { "👤" };
                let mode = format!("{}", p.participation_mode());
                ListItem::new(format!("{} {} - {}", role_icon, p.name(), mode))
            })
            .collect()
    } else {
        vec![ListItem::new("No participants")]
    };

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Participants"));

    f.render_widget(list, area);
}

fn render_help(f: &mut Frame, area: Rect) {
    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Tab / →", Style::default().fg(Color::Yellow)),
            Span::raw("  Next tab"),
        ]),
        Line::from(vec![
            Span::styled("Shift+Tab / ←", Style::default().fg(Color::Yellow)),
            Span::raw("  Previous tab"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("j / ↓", Style::default().fg(Color::Yellow)),
            Span::raw("  Scroll down"),
        ]),
        Line::from(vec![
            Span::styled("k / ↑", Style::default().fg(Color::Yellow)),
            Span::raw("  Scroll up"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("q / Esc", Style::default().fg(Color::Yellow)),
            Span::raw("  Quit"),
        ]),
        Line::from(""),
        Line::from(""),
        Line::from(vec![
            Span::styled("💡 Tip: ", Style::default().fg(Color::Yellow)),
            Span::raw("You can select text in the terminal to copy the Session ID"),
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
        Span::raw("Press "),
        Span::styled("q", Style::default().fg(Color::Yellow)),
        Span::raw(" to quit | "),
        Span::styled("Tab", Style::default().fg(Color::Yellow)),
        Span::raw(" to switch tabs | "),
        Span::styled("j/k", Style::default().fg(Color::Yellow)),
        Span::raw(" to scroll | Mouse selection enabled"),
    ]);

    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Gray));

    f.render_widget(paragraph, area);
}
