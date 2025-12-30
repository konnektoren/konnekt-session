use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

pub fn render_help(f: &mut Frame, area: Rect) {
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
            "Results Tab:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  j/k", Style::default().fg(Color::Yellow)),
            Span::raw("  Navigate completed activities"),
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
