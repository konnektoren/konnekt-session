use crossterm::event::{self, Event, KeyCode, KeyEvent};
use std::time::Duration;

pub enum AppEvent {
    Key(KeyCode),
    Tick,
}

pub async fn read_events() -> std::io::Result<AppEvent> {
    // Poll for events with timeout
    if event::poll(Duration::from_millis(100))? {
        match event::read()? {
            Event::Key(KeyEvent { code, .. }) => Ok(AppEvent::Key(code)),
            _ => Ok(AppEvent::Tick),
        }
    } else {
        Ok(AppEvent::Tick)
    }
}
