use super::app::App;
use ratatui::Frame;
use ratatui::layout::Rect;

mod activities;
mod events;
mod footer;
mod header;
mod help;
mod lobby;
mod participants;
mod results;
mod session;

use activities::render_activities;
use events::render_events;
use footer::render_footer;
use header::render_header;
use help::render_help;
use lobby::render_lobby;
use participants::render_participants;
use results::render_results;
use session::render_session;

use super::app::Tab;
use ratatui::layout::{Constraint, Direction, Layout};

/// Main render function - orchestrates all tabs
pub fn render(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Footer
        ])
        .split(f.area());

    header::render_header(f, chunks[0], app);
    render_content(f, chunks[1], app);
    footer::render_footer(f, chunks[2], app);
}

/// Route to appropriate tab renderer
fn render_content(f: &mut Frame, area: Rect, app: &App) {
    match app.current_tab {
        Tab::Session => session::render_session(f, area, app),
        Tab::Lobby => lobby::render_lobby(f, area, app),
        Tab::Activities => activities::render_activities(f, area, app),
        Tab::Participants => participants::render_participants(f, area, app),
        Tab::Results => results::render_results(f, area, app),
        Tab::Events => events::render_events(f, area, app),
        Tab::Help => help::render_help(f, area),
    }
}
