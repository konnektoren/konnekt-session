#[cfg(test)]
mod test {
    use futures::{select, FutureExt};
    use futures_timer::Delay;
    use matchbox_socket::{PeerState, WebRtcSocket};
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use wasm_bindgen_test::*;
    use web_sys::console;

    /// To run the tests:
    ///
    /// # Run with Firefox in headless mode (CI/CD):
    /// wasm-pack test --firefox --headless
    ///

    wasm_bindgen_test_configure!(run_in_browser);

    fn log(msg: &str) {
        console::log_1(&msg.into());
    }

    fn error(msg: &str) {
        console::error_1(&msg.into());
    }

    async fn run_client(
        url: String,
        message: String,
        received_messages: Arc<Mutex<HashMap<String, String>>>,
    ) -> Result<(), String> {
        log("Client starting");
        let (mut socket, loop_fut) = WebRtcSocket::new_reliable(url);
        let mut loop_fut = loop_fut.fuse();

        let mut timeout = Delay::new(Duration::from_millis(100)).fuse();
        let mut test_timeout = Delay::new(Duration::from_secs(10)).fuse();

        loop {
            // Handle peer updates
            for (peer, state) in socket.update_peers() {
                match state {
                    PeerState::Connected => {
                        log(&format!("➡️ Peer connected: {}", peer));
                        let packet = message.as_bytes().to_vec().into_boxed_slice();
                        socket.channel_mut(0).send(packet, peer);
                    }
                    PeerState::Disconnected => {
                        log(&format!("⬅️ Peer disconnected: {}", peer));
                    }
                }
            }

            // Handle incoming messages
            for (peer, packet) in socket.channel_mut(0).receive() {
                let message = String::from_utf8_lossy(&packet).to_string();
                log(&format!("📨 Message received: {}", message));
                received_messages
                    .lock()
                    .unwrap()
                    .insert(peer.to_string(), message);
                return Ok(());
            }

            select! {
                _ = &mut test_timeout => {
                    log("❌ Test timeout");
                    return Err("Test timeout reached".to_string());
                }
                _ = &mut timeout => {
                    timeout = Delay::new(Duration::from_millis(100)).fuse();
                }
                _ = &mut loop_fut => {
                    log("✅ Loop completed");
                    return Ok(());
                }
            }
        }
    }

    #[wasm_bindgen_test]
    async fn test_matchbox_connection() {
        console_error_panic_hook::set_once();

        log("🏁 Test 'test_matchbox_connection' starting");

        let room_id = "ac2965a1-87c2-432a-aa9f-1f63319b193b".to_string();
        let base_url = "wss://match.konnektoren.help";
        let room_url = format!("{}/{}", base_url, room_id);

        log(&format!("🔌 Connecting to room: {}", room_id));

        let messages = Arc::new(Mutex::new(HashMap::new()));

        // Run single client
        let result = run_client(
            room_url.clone(),
            "hello from test client!".to_string(),
            messages.clone(),
        )
        .await;

        // Check result
        match result {
            Ok(_) => log("✅ Client completed successfully"),
            Err(ref e) => error(&format!("❌ Client error: {}", e)),
        }
        result.expect("Client should complete successfully");

        // Get final messages
        let received = messages.lock().unwrap();

        // Print received messages
        for (peer, msg) in received.iter() {
            log(&format!("📝 Final message from {}: {}", peer, msg));
        }

        assert!(
            !received.is_empty(),
            "Should have received at least one message"
        );

        log("🏆 Test completed successfully!");
    }
}
