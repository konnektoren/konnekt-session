//! Component previews for konnekt-session-yew using yew-preview.
//!
//! Collects all component previews and groups them by category.

use konnekt_session_core::{Lobby, domain::Participant};
use yew_preview::create_component_group;
use yew_preview::prelude::*;

use crate::components::{
    ActivityList, ParticipantList, ResultsView, SessionInfo, SubmissionStatus,
};

// ── Fixture helpers ──────────────────────────────────────────────────────────

fn _make_lobby() -> Lobby {
    let host = Participant::new_host("Alice".to_string()).unwrap();
    let mut lobby = Lobby::new("Preview Lobby".to_string(), host).unwrap();
    lobby
        .add_guest(Participant::new_guest("Bob".to_string()).unwrap())
        .unwrap();
    lobby
        .add_guest(Participant::new_guest("Charlie".to_string()).unwrap())
        .unwrap();
    lobby
}

/// Collect all component previews into organized groups
pub fn preview_groups() -> ComponentList {
    vec![
        create_component_group!("Session", SessionInfo::preview()),
        create_component_group!("Lobby", ParticipantList::preview(), ActivityList::preview(),),
        create_component_group!(
            "Activity",
            ResultsView::preview(),
            SubmissionStatus::preview(),
        ),
    ]
}
