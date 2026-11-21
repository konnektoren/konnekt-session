use konnekt_session_p2p::protocol::identity::{AwaitingChallenge, IdentityVerification};
use konnekt_session_p2p::protocol::join::{Connecting, JoinSession};
use konnekt_session_rating::protocol::rating_match::{Playing, RatingMatch};
use std::{env, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=../konnekt-session-p2p/src/protocol/join.rs");
    println!("cargo:rerun-if-changed=../konnekt-session-p2p/src/protocol/identity.rs");
    println!("cargo:rerun-if-changed=../konnekt-session-rating/src/protocol/rating_match.rs");
    let output = PathBuf::from(env::var_os("OUT_DIR").ok_or("OUT_DIR is missing")?);
    fs::write(
        output.join("join.mmd"),
        JoinSession::<Connecting>::schema().to_mermaid(),
    )?;
    fs::write(
        output.join("identity.mmd"),
        IdentityVerification::<AwaitingChallenge>::schema().to_mermaid(),
    )?;
    fs::write(
        output.join("rating-match.mmd"),
        RatingMatch::<Playing>::schema().to_mermaid(),
    )?;
    Ok(())
}
