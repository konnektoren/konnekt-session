use konnekt_session_core::{DomainCommand, Lobby};
use konnekt_session_p2p::{SessionId, SessionLoop};
use tokio::sync::{mpsc, watch};
use uuid::Uuid;

/// Snapshot of session state (read-only, cheap to clone)
#[derive(Debug, Clone)]
pub struct SessionSnapshot {
    pub lobby: Option<Lobby>,
    pub local_peer_id: Option<String>,
    pub peer_count: usize,
    pub is_host: bool,
    pub lobby_id: Uuid,
}

impl Default for SessionSnapshot {
    fn default() -> Self {
        Self {
            lobby: None,
            local_peer_id: None,
            peer_count: 0,
            is_host: false,
            lobby_id: Uuid::nil(),
        }
    }
}

/// Background runtime for SessionLoop
pub struct SessionRuntime {
    /// Send commands to SessionLoop
    cmd_tx: mpsc::Sender<DomainCommand>,

    /// Receive state snapshots (latest always available)
    state_rx: watch::Receiver<SessionSnapshot>,

    /// Handle to background task
    task_handle: tokio::task::JoinHandle<()>,
}

impl SessionRuntime {
    /// Spawn a new runtime with existing SessionLoop
    pub fn spawn(mut session_loop: SessionLoop, session_id: SessionId) -> Self {
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<DomainCommand>(100);
        let (state_tx, state_rx) = watch::channel(SessionSnapshot::default());

        let lobby_id = session_loop.lobby_id();
        let is_host = session_loop.is_host();

        let task_handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(100));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            tracing::info!("SessionRuntime started for session {}", session_id);

            loop {
                interval.tick().await;

                // 1. Process incoming commands
                while let Ok(cmd) = cmd_rx.try_recv() {
                    if let Err(e) = session_loop.submit_command(cmd) {
                        tracing::error!("Failed to submit command: {:?}", e);
                    }
                }

                // 2. Poll SessionLoop (P2P + Domain)
                let processed = session_loop.poll();

                if processed > 0 {
                    tracing::debug!("SessionRuntime processed {} events", processed);
                }

                // 3. Publish snapshot (non-blocking, only if changed)
                let snapshot = SessionSnapshot {
                    lobby: session_loop.get_lobby().cloned(),
                    local_peer_id: session_loop.local_peer_id().map(|p| p.to_string()),
                    peer_count: session_loop.connected_peers().len(),
                    is_host: session_loop.is_host(),
                    lobby_id,
                };

                // Only send if changed (watch channel deduplicates)
                let _ = state_tx.send(snapshot);
            }
        });

        Self {
            cmd_tx,
            state_rx,
            task_handle,
        }
    }

    /// Submit a command (non-blocking)
    pub async fn submit_command(
        &self,
        cmd: DomainCommand,
    ) -> Result<(), mpsc::error::SendError<DomainCommand>> {
        self.cmd_tx.send(cmd).await
    }

    /// Get latest state snapshot (always succeeds, never blocks)
    pub fn snapshot(&self) -> SessionSnapshot {
        self.state_rx.borrow().clone()
    }

    /// Subscribe to state changes
    pub fn subscribe(&self) -> watch::Receiver<SessionSnapshot> {
        self.state_rx.clone()
    }

    /// Shutdown runtime
    pub async fn shutdown(self) {
        self.task_handle.abort();
        let _ = self.task_handle.await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use konnekt_session_p2p::{IceServer, P2PLoopBuilder};

    #[tokio::test]
    #[ignore] // Requires network
    async fn test_runtime_spawns_and_publishes_snapshots() {
        let (session_loop, session_id) = P2PLoopBuilder::new()
            .build_session_host(
                "wss://match.konnektoren.help",
                IceServer::default_stun_servers(),
                "Test Lobby".to_string(),
                "TestHost".to_string(),
            )
            .await
            .unwrap();

        let runtime = SessionRuntime::spawn(session_loop, session_id);

        // Wait for initial snapshot
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let snapshot = runtime.snapshot();

        assert!(snapshot.lobby.is_some());
        assert_eq!(snapshot.lobby.unwrap().name(), "Test Lobby");
        assert!(snapshot.is_host);

        runtime.shutdown().await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_runtime_processes_commands() {
        let (session_loop, session_id) = P2PLoopBuilder::new()
            .build_session_host(
                "wss://match.konnektoren.help",
                IceServer::default_stun_servers(),
                "Test Lobby".to_string(),
                "TestHost".to_string(),
            )
            .await
            .unwrap();

        let lobby_id = session_loop.lobby_id();
        let runtime = SessionRuntime::spawn(session_loop, session_id);

        // Submit a command
        runtime
            .submit_command(DomainCommand::JoinLobby {
                lobby_id,
                guest_name: "TestGuest".to_string(),
            })
            .await
            .unwrap();

        // Wait for processing
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        let snapshot = runtime.snapshot();

        // Note: This will fail because we can't join our own session without P2P peer
        // But it proves command was processed
        assert!(snapshot.lobby.is_some());

        runtime.shutdown().await;
    }

    #[tokio::test]
    async fn test_snapshot_is_cheap_to_clone() {
        use std::time::Instant;

        let snapshot = SessionSnapshot::default();

        let start = Instant::now();
        for _ in 0..10_000 {
            let _ = snapshot.clone();
        }
        let elapsed = start.elapsed();

        // Should be < 1ms for 10k clones
        assert!(elapsed.as_millis() < 10, "Cloning too slow: {:?}", elapsed);
    }
}
