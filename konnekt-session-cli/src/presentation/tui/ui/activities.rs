use crate::presentation::tui::app::{ActivitiesTab, App};
use konnekt_session_core::EchoChallenge;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

pub fn render_activities(f: &mut Frame, area: Rect, app: &App) {
    let activities_tab = &app.activities_tab;

    if activities_tab.is_host() {
        render_activities_host(f, area, activities_tab);
    } else {
        render_activities_guest(f, area, activities_tab);
    }
}

fn render_activities_host(f: &mut Frame, area: Rect, activities_tab: &ActivitiesTab) {
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

fn render_activities_guest(f: &mut Frame, area: Rect, activities_tab: &ActivitiesTab) {
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
                    challenge.prompt.clone(),
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
