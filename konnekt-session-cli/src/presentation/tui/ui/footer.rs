use crate::presentation::tui::app::{App, Tab};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
};

pub fn render_footer(f: &mut Frame, area: Rect, app: &App) {
    let shortcuts = match app.current_tab {
        Tab::Session => "y: copy ID | c: copy cmd | Tab: switch | q: quit",
        Tab::Activities if app.is_host && app.activities_tab.current_activity().is_none() => {
            // Host in planning mode (no activity running)
            "j/k: select | p: plan | s: start | Tab: switch | q: quit"
        }
        Tab::Activities if app.is_host && app.activities_tab.current_activity().is_some() => {
            // Host during activity (can answer + cancel)
            "Type answer | Enter: submit | x: cancel | Tab: switch | q: quit"
        }
        Tab::Activities => {
            // Guest during activity (can only answer)
            "Type answer | Enter: submit | Tab: switch | q: quit"
        }
        Tab::Participants if app.is_host => {
            "j/k: select | t: toggle mode | x: kick | Tab: switch | q: quit"
        }
        Tab::Participants => "t: toggle mode | Tab: switch | q: quit",
        Tab::Results => "j/k: navigate | Tab: switch | q: quit",
        _ => "Tab: switch | q: quit",
    };

    let text = Line::from(shortcuts);

    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Gray));

    f.render_widget(paragraph, area);
}
