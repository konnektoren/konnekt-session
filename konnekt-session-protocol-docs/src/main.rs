use std::{error::Error, fs, path::Path};

fn main() -> Result<(), Box<dyn Error>> {
    let directory =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../konnekt-session-workspace/protocols");
    let diagrams = [
        (
            "Event log reconciliation",
            "event-log-reconciliation",
            include_str!(concat!(env!("OUT_DIR"), "/event-log-reconciliation.mmd")),
        ),
        (
            "Host failover",
            "host-failover",
            include_str!(concat!(env!("OUT_DIR"), "/host-failover.mmd")),
        ),
        (
            "Asset transfer",
            "asset-transfer",
            include_str!(concat!(env!("OUT_DIR"), "/asset-transfer.mmd")),
        ),
        (
            "Synchronized start",
            "synchronized-start",
            include_str!(concat!(env!("OUT_DIR"), "/synchronized-start.mmd")),
        ),
        (
            "Clock probe",
            "clock-probe",
            include_str!(concat!(env!("OUT_DIR"), "/clock-probe.mmd")),
        ),
        (
            "Player readiness",
            "player-readiness",
            include_str!(concat!(env!("OUT_DIR"), "/player-readiness.mmd")),
        ),
        (
            "Activity run",
            "activity-run",
            include_str!(concat!(env!("OUT_DIR"), "/activity-run.mmd")),
        ),
        (
            "Command outcome",
            "command-outcome",
            include_str!(concat!(env!("OUT_DIR"), "/command-outcome.mmd")),
        ),
        (
            "Request execution",
            "request-execution",
            include_str!(concat!(env!("OUT_DIR"), "/request-execution.mmd")),
        ),
        (
            "Identity verification",
            "identity-verification",
            include_str!(concat!(env!("OUT_DIR"), "/identity.mmd")),
        ),
        (
            "Peer grace period",
            "peer-grace",
            include_str!(concat!(env!("OUT_DIR"), "/peer-grace.mmd")),
        ),
        (
            "Join session",
            "join-session",
            include_str!(concat!(env!("OUT_DIR"), "/join.mmd")),
        ),
        (
            "State synchronization",
            "state-synchronization",
            include_str!(concat!(env!("OUT_DIR"), "/synchronization.mmd")),
        ),
        (
            "Rating match",
            "rating-match",
            include_str!(concat!(env!("OUT_DIR"), "/rating-match.mmd")),
        ),
    ];
    let mut section = String::from(
        "<!-- generated-protocol-diagrams:start -->\n## Generated state machines\n\nRun `just protocol-diagrams` to update these diagrams.\n",
    );
    for (title, name, diagram) in diagrams {
        let content = format!(
            "---\ntitle: {title} diagram\ntype: protocol\n---\n\n# {title}\n\n```mermaid\n{}\n```\n",
            diagram.trim_end()
        );
        fs::write(directory.join(format!("{name}-diagram.md")), content)?;
        section.push_str(&format!(
            "\n### {title}\n\n```mermaid\n{}\n```\n",
            diagram.trim_end()
        ));
    }
    section.push_str("<!-- generated-protocol-diagrams:end -->");
    let index_path = directory.join("index.md");
    let index = fs::read_to_string(&index_path)?;
    let start = "<!-- generated-protocol-diagrams:start -->";
    let end = "<!-- generated-protocol-diagrams:end -->";
    let updated = match (index.find(start), index.find(end)) {
        (Some(a), Some(b)) if a <= b => {
            format!("{}{}{}", &index[..a], section, &index[b + end.len()..])
        }
        (None, None) => format!("{}\n\n{}\n", index.trim_end(), section),
        _ => return Err("invalid generated diagram markers in protocol index".into()),
    };
    fs::write(index_path, updated)?;
    Ok(())
}
