use super::app::{App, Tab};
use konnekt_session_core::EchoChallenge;
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
    render_footer(f, chunks[2], app);
}

fn render_header(f: &mut Frame, area: Rect, app: &App) {
    let titles = vec![
        Tab::Session.title(),
        Tab::Lobby.title(),
        Tab::Activities.title(),
        Tab::Participants.title(),
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

fn render_content(f: &mut Frame, area: Rect, app: &App) {
    match app.current_tab {
        Tab::Session => render_session(f, area, app),
        Tab::Lobby => render_lobby(f, area, app),
        Tab::Activities => render_activities(f, area, app),
        Tab::Participants => render_participants(f, area, app),
        Tab::Events => render_events(f, area, app),
        Tab::Help => render_help(f, area),
    }
}

fn render_session(f: &mut Frame, area: Rect, app: &App) {
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

fn render_lobby(f: &mut Frame, area: Rect, app: &App) {
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

fn render_activities(f: &mut Frame, area: Rect, app: &App) {
    let activities_tab = &app.activities_tab;

    if activities_tab.is_host() {
        render_activities_host(f, area, activities_tab);
    } else {
        render_activities_guest(f, area, activities_tab);
    }
}

fn render_activities_host(f: &mut Frame, area: Rect, activities_tab: &super::app::ActivitiesTab) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50), // Available templates
            Constraint::Percentage(50), // Planned/running activities
        ])
        .split(area);

    // Available templates
    let template_items: Vec<ListItem> = activities_tab
        .available_activities()
        .iter()
        .enumerate()
        .map(|(idx, template)| {
            let prefix = if idx == activities_tab.selected_template() {
                "> "
            } else {
                "  "
            };

            let mut item = ListItem::new(Line::from(vec![
                Span::raw(prefix),
                Span::styled(&template.name, Style::default().fg(Color::Cyan)),
            ]));

            if idx == activities_tab.selected_template() {
                item = item.style(Style::default().bg(Color::DarkGray));
            }

            item
        })
        .collect();

    let templates_list = List::new(template_items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Available Activities (p: plan, j/k: select)"),
    );

    f.render_widget(templates_list, chunks[0]);

    // Planned/running activities
    let mut activity_text = vec![];

    if let Some(current) = activities_tab.current_activity() {
        activity_text.push(Line::from(vec![Span::styled(
            "🎮 Current Activity:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]));
        activity_text.push(Line::from(""));
        activity_text.push(Line::from(vec![Span::styled(
            &current.name,
            Style::default().fg(Color::Yellow),
        )]));
        activity_text.push(Line::from(""));
        activity_text.push(Line::from(vec![
            Span::raw("Press "),
            Span::styled("x", Style::default().fg(Color::Red)),
            Span::raw(" to cancel"),
        ]));
    } else if !activities_tab.planned_activities().is_empty() {
        activity_text.push(Line::from(vec![Span::styled(
            "📋 Planned Activities:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]));
        activity_text.push(Line::from(""));

        for activity in activities_tab.planned_activities() {
            activity_text.push(Line::from(vec![
                Span::raw("  • "),
                Span::styled(&activity.name, Style::default().fg(Color::White)),
            ]));
        }

        activity_text.push(Line::from(""));
        activity_text.push(Line::from(vec![
            Span::raw("Press "),
            Span::styled("s", Style::default().fg(Color::Green)),
            Span::raw(" to start first activity"),
        ]));
    } else {
        activity_text.push(Line::from("No activities planned"));
        activity_text.push(Line::from(""));
        activity_text.push(Line::from(vec![
            Span::raw("Press "),
            Span::styled("p", Style::default().fg(Color::Green)),
            Span::raw(" to plan selected activity"),
        ]));
    }

    let activities_para = Paragraph::new(activity_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Activity Queue"),
    );

    f.render_widget(activities_para, chunks[1]);
}

fn render_activities_guest(f: &mut Frame, area: Rect, activities_tab: &super::app::ActivitiesTab) {
    let mut text = vec![];

    if let Some(current) = activities_tab.current_activity() {
        text.push(Line::from(vec![Span::styled(
            "🎮 Current Activity:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]));
        text.push(Line::from(""));
        text.push(Line::from(vec![Span::styled(
            &current.name,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]));
        text.push(Line::from(""));

        // Parse activity config to show prompt
        if let Ok(challenge) = EchoChallenge::from_config(current.config.clone()) {
            text.push(Line::from(vec![
                Span::styled("Prompt: ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    challenge.prompt.clone(), // 🔧 FIX: Clone instead of borrowing
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            text.push(Line::from(""));
        }

        text.push(Line::from("─".repeat(50)));
        text.push(Line::from(""));
        text.push(Line::from(vec![
            Span::styled("Your Response: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                activities_tab.activity_input(),
                Style::default().fg(Color::Green),
            ),
        ]));
        text.push(Line::from(""));
        text.push(Line::from(vec![
            Span::raw("Press "),
            Span::styled("Enter", Style::default().fg(Color::Green)),
            Span::raw(" to submit"),
        ]));
    } else if !activities_tab.planned_activities().is_empty() {
        text.push(Line::from(vec![Span::styled(
            "📋 Upcoming Activities:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]));
        text.push(Line::from(""));

        for activity in activities_tab.planned_activities() {
            text.push(Line::from(vec![
                Span::raw("  • "),
                Span::styled(&activity.name, Style::default().fg(Color::White)),
            ]));
        }

        text.push(Line::from(""));
        text.push(Line::from("Waiting for host to start..."));
    } else {
        text.push(Line::from("No activities available"));
        text.push(Line::from(""));
        text.push(Line::from("Waiting for host to plan activities..."));
    }

    let paragraph =
        Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("Activities"));

    f.render_widget(paragraph, area);
}

fn render_events(f: &mut Frame, area: Rect, app: &App) {
    let events_tab = &app.events_tab;

    let events: Vec<ListItem> = events_tab
        .event_log()
        .iter()
        .skip(events_tab.scroll_offset())
        .map(|e| ListItem::new(e.as_str()))
        .collect();

    let list = List::new(events)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Event Log ({})", events_tab.event_log().len())),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(list, area);
}

fn render_participants(f: &mut Frame, area: Rect, app: &App) {
    let participants_tab = &app.participants_tab;

    let items: Vec<ListItem> = if let Some(lobby) = &app.lobby_snapshot {
        lobby
            .participants()
            .values()
            .enumerate()
            .map(|(idx, p)| {
                let role_icon = if p.is_host() { "👑" } else { "👤" };

                let (mode_text, mode_style) = match p.participation_mode() {
                    konnekt_session_core::ParticipationMode::Active => {
                        ("🎮 Active", Style::default().fg(Color::Green))
                    }
                    konnekt_session_core::ParticipationMode::Spectating => {
                        ("👁️  Spectating", Style::default().fg(Color::Yellow))
                    }
                };

                let selected = app.is_host
                    && app.current_tab == Tab::Participants
                    && idx == participants_tab.selected_participant();

                let prefix = if selected { "> " } else { "  " };

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

    let title = if app.is_host {
        "Participants (j/k: select, t: toggle mode, x: kick)"
    } else {
        "Participants (t: toggle your mode)"
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
            "Activities Tab (Host):",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  j/k", Style::default().fg(Color::Yellow)),
            Span::raw("  Select activity template"),
        ]),
        Line::from(vec![
            Span::styled("  p", Style::default().fg(Color::Yellow)),
            Span::raw("  Plan selected activity"),
        ]),
        Line::from(vec![
            Span::styled("  s", Style::default().fg(Color::Yellow)),
            Span::raw("  Start first planned activity"),
        ]),
        Line::from(vec![
            Span::styled("  x", Style::default().fg(Color::Yellow)),
            Span::raw("  Cancel current activity"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Activities Tab (Guest):",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  Type", Style::default().fg(Color::Yellow)),
            Span::raw("  Enter your response"),
        ]),
        Line::from(vec![
            Span::styled("  Enter", Style::default().fg(Color::Yellow)),
            Span::raw("  Submit response"),
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

fn render_footer(f: &mut Frame, area: Rect, app: &App) {
    let shortcuts = match app.current_tab {
        Tab::Session => "y: copy ID | c: copy cmd | Tab: switch | q: quit",
        Tab::Activities if app.is_host => {
            "j/k: select | p: plan | s: start | x: cancel | Tab: switch | q: quit"
        }
        Tab::Activities => "Type response | Enter: submit | Tab: switch | q: quit",
        Tab::Participants if app.is_host => {
            "j/k: select | t: toggle mode | x: kick | Tab: switch | q: quit"
        }
        Tab::Participants => "t: toggle mode | Tab: switch | q: quit",
        _ => "Tab: switch | q: quit",
    };

    let text = Line::from(shortcuts);

    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Gray));

    f.render_widget(paragraph, area);
}
