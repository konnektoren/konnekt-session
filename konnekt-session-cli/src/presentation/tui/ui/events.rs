use crate::presentation::tui::app::App;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem},
};

pub fn render_events(f: &mut Frame, area: Rect, app: &App) {
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
