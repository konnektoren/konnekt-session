use konnekt_session_core::protocol::activity_run::{ActivityRunLifecycle, InProgress};
use konnekt_session_p2p::protocol::identity::{AwaitingChallenge, IdentityVerification};
use konnekt_session_p2p::protocol::join::{Connecting, JoinSession};
use konnekt_session_p2p::protocol::request_ledger::{Pending, RequestExecution};
use konnekt_session_p2p::protocol::synchronization::{AwaitingSnapshot, StateSynchronization};
use konnekt_session_rating::protocol::rating_match::{Playing, RatingMatch};
use std::{env, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=../konnekt-session-core/src/protocol/activity_run.rs");
    println!("cargo:rerun-if-changed=../konnekt-session-p2p/src/protocol/command_outcome.rs");
    println!("cargo:rerun-if-changed=../konnekt-session-p2p/src/protocol/join.rs");
    println!("cargo:rerun-if-changed=../konnekt-session-p2p/src/protocol/request_ledger.rs");
    println!("cargo:rerun-if-changed=../konnekt-session-p2p/src/protocol/identity.rs");
    println!("cargo:rerun-if-changed=../konnekt-session-p2p/src/protocol/synchronization.rs");
    println!("cargo:rerun-if-changed=../konnekt-session-rating/src/protocol/rating_match.rs");
    let output = PathBuf::from(env::var_os("OUT_DIR").ok_or("OUT_DIR is missing")?);
    fs::write(
        output.join("activity-run.mmd"),
        ActivityRunLifecycle::<InProgress>::schema().to_mermaid(),
    )?;
    fs::write(
        output.join("command-outcome.mmd"),
        CommandOutcome::<Prepared>::schema().to_mermaid(),
    )?;
    fs::write(
        output.join("request-execution.mmd"),
        RequestExecution::<Pending>::schema().to_mermaid(),
    )?;
    fs::write(
        output.join("join.mmd"),
        JoinSession::<Connecting>::schema().to_mermaid(),
    )?;
    fs::write(
        output.join("identity.mmd"),
        IdentityVerification::<AwaitingChallenge>::schema().to_mermaid(),
    )?;
    fs::write(
        output.join("synchronization.mmd"),
        StateSynchronization::<AwaitingSnapshot>::schema().to_mermaid(),
    )?;
    fs::write(
        output.join("rating-match.mmd"),
        RatingMatch::<Playing>::schema().to_mermaid(),
    )?;
    Ok(())
}
use konnekt_session_p2p::protocol::command_outcome::{CommandOutcome, Prepared};
