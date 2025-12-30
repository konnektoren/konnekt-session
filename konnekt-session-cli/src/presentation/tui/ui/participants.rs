use crate::presentation::tui::app::{App, Tab};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
};

pub fn render_participants(f: &mut Frame, area: Rect, app: &App) {
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
