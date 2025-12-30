use crate::presentation::tui::app::App;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

pub fn render_results(f: &mut Frame, area: Rect, app: &App) {
    let results_tab = &app.results_tab;

    if results_tab.completed_activities().is_empty() {
        let text = vec![
            Line::from("No completed activities yet"),
            Line::from(""),
            Line::from("Complete some activities to see results here!"),
        ];

        let paragraph = Paragraph::new(text).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Activity Results"),
        );

        f.render_widget(paragraph, area);
        return;
    }

    // Split area: activity list on left, details on right
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    // Render activity list
    let activity_items: Vec<ListItem> = results_tab
        .completed_activities()
        .iter()
        .enumerate()
        .map(|(idx, activity)| {
            let prefix = if idx == results_tab.selected_activity() {
                "> "
            } else {
                "  "
            };

            let mut item = ListItem::new(Line::from(vec![
                Span::raw(prefix),
                Span::styled(&activity.activity_name, Style::default().fg(Color::Cyan)),
                Span::raw(format!(" ({} results)", activity.results.len())),
            ]));

            if idx == results_tab.selected_activity() {
                item = item.style(Style::default().bg(Color::DarkGray));
            }

            item
        })
        .collect();

    let activities_list = List::new(activity_items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Completed Activities (j/k: select)"),
    );

    f.render_widget(activities_list, chunks[0]);

    // Render selected activity details
    if let Some(selected) = results_tab
        .completed_activities()
        .get(results_tab.selected_activity())
    {
        let mut text = vec![
            Line::from(vec![Span::styled(
                &selected.activity_name,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from("─".repeat(50)),
            Line::from(""),
        ];

        // Sort results by score (descending)
        let mut sorted_results = selected.results.clone();
        sorted_results.sort_by(|a, b| {
            b.score
                .unwrap_or(0)
                .cmp(&a.score.unwrap_or(0))
                .then_with(|| {
                    a.time_ms
                        .unwrap_or(u64::MAX)
                        .cmp(&b.time_ms.unwrap_or(u64::MAX))
                })
        });

        for (idx, result) in sorted_results.iter().enumerate() {
            let rank_icon = match idx {
                0 => "🥇",
                1 => "🥈",
                2 => "🥉",
                _ => "  ",
            };

            text.push(Line::from(vec![
                Span::raw(format!("{} ", rank_icon)),
                Span::styled(
                    &result.participant_name,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));

            if let Some(response) = &result.response {
                text.push(Line::from(vec![
                    Span::styled("   Response: ", Style::default().fg(Color::Gray)),
                    Span::styled(response, Style::default().fg(Color::Green)),
                ]));
            }

            if let Some(score) = result.score {
                text.push(Line::from(vec![
                    Span::styled("   Score: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        format!("{}", score),
                        if score == 100 {
                            Style::default().fg(Color::Green)
                        } else {
                            Style::default().fg(Color::Yellow)
                        },
                    ),
                ]));
            }

            if let Some(time_ms) = result.time_ms {
                text.push(Line::from(vec![
                    Span::styled("   Time: ", Style::default().fg(Color::Gray)),
                    Span::raw(format!("{}ms", time_ms)),
                ]));
            }

            text.push(Line::from(""));
        }

        let details = Paragraph::new(text).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Results (sorted by score)"),
        );

        f.render_widget(details, chunks[1]);
    }
}
