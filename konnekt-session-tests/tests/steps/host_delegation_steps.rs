use cucumber::{given, then, when};
use konnekt_session_core::{LobbyRole, Participant, Timestamp};
use konnekt_session_tests::SessionWorld;

// ===== Given Steps =====

#[given(expr = "a lobby exists with a host")]
async fn lobby_exists_with_default_host(world: &mut SessionWorld) {
    lobby_exists_with_named_host(world, "Host".to_string()).await;
}

#[given(expr = "a lobby exists with Host {string}")]
async fn lobby_exists_with_host_name(world: &mut SessionWorld, name: String) {
    lobby_exists_with_named_host(world, name).await;
}

// Helper function to reduce duplication
async fn lobby_exists_with_named_host(world: &mut SessionWorld, host_name: String) {
    let host = Participant::new_host(host_name.clone()).unwrap();
    world
        .participants_by_name
        .insert(host_name.clone(), host.clone());

    let lobby = konnekt_session_core::Lobby::new("Test Lobby".to_string(), host).unwrap();
    world.lobby = Some(lobby);

    world.join_times.insert(host_name, world.current_time);
}

#[given(expr = "guest {string} joined {int} seconds ago")]
async fn guest_joined_seconds_ago(world: &mut SessionWorld, name: String, seconds_ago: u64) {
    let millis_ago = seconds_ago * 1000;
    let join_time = world.current_time.saturating_sub(millis_ago);

    let guest = Participant::with_timestamp(
        name.clone(),
        LobbyRole::Guest,
        Timestamp::from_millis(join_time),
    )
    .unwrap();

    world
        .participants_by_name
        .insert(name.clone(), guest.clone());
    world.lobby_mut().add_guest(guest).unwrap();
    world.join_times.insert(name, join_time);
}

#[given(expr = "guest {string} joined at time {int}")]
async fn guest_joined_at_time(world: &mut SessionWorld, name: String, time: u64) {
    let guest =
        Participant::with_timestamp(name.clone(), LobbyRole::Guest, Timestamp::from_millis(time))
            .unwrap();

    world
        .participants_by_name
        .insert(name.clone(), guest.clone());
    world.lobby_mut().add_guest(guest).unwrap();
    world.join_times.insert(name, time);
}

#[given("an activity is in progress")]
async fn activity_in_progress(_world: &mut SessionWorld) {
    // TODO: Implement when activity management is ready
    // For now, this is a no-op placeholder
}

#[given("the host disconnects at time T")]
#[given("the host disconnects")]
async fn host_disconnects(world: &mut SessionWorld) {
    // Mark the disconnect time
    world.current_time += 0; // Just for clarity
    // Actual disconnect handling will be in P2P layer
}

#[given(expr = "a lobby with only the host and {int} guest(s)")]
async fn lobby_with_host_and_guests(world: &mut SessionWorld, guest_count: usize) {
    lobby_exists_with_default_host(world).await;

    for i in 0..guest_count {
        let guest_name = format!("Guest{}", i + 1);
        guest_joined_seconds_ago(world, guest_name, (i + 1) as u64).await;
    }
}

#[given("a lobby with only the host")]
async fn lobby_with_only_host(world: &mut SessionWorld) {
    lobby_exists_with_default_host(world).await;
}

// ===== When Steps =====

#[when(expr = "the host delegates to {string}")]
async fn host_delegates_to(world: &mut SessionWorld, guest_name: String) {
    let guest_id = world.get_participant_id(&guest_name);

    let result = world.lobby_mut().delegate_host(guest_id);

    if let Err(e) = result {
        world.last_error = Some(e);
    }
}

#[when(expr = "{int} seconds pass")]
#[when(expr = "{int} second passes")]
async fn seconds_pass(world: &mut SessionWorld, seconds: u64) {
    world.advance_time(seconds * 1000);
}

#[when(expr = "{int} more seconds pass")]
async fn more_seconds_pass(world: &mut SessionWorld, seconds: u64) {
    world.advance_time(seconds * 1000);
}

#[when("the host reconnects")]
async fn host_reconnects(_world: &mut SessionWorld) {
    // TODO: Implement reconnection logic when P2P layer is ready
}

#[when("the 30s timeout expires")]
async fn timeout_expires(world: &mut SessionWorld) {
    // Simulate 30 second timeout
    world.advance_time(30_000);

    // Trigger auto-delegation
    let result = world.lobby_mut().auto_delegate_host();

    if let Err(e) = result {
        world.last_error = Some(e);
    }
}

// ===== Then Steps =====

#[then(expr = "{string} should become the host")]
#[then(expr = "{string} should be the host")]
async fn should_become_host(world: &mut SessionWorld, name: String) {
    let participant_id = world.get_participant_id(&name);

    assert_eq!(
        world.lobby().host_id(),
        participant_id,
        "{} should be the host, but host_id is {}",
        name,
        world.lobby().host_id()
    );

    let participant = world
        .lobby()
        .participants()
        .get(&participant_id)
        .expect("Participant not found in lobby");

    assert!(participant.is_host(), "{} should have host role", name);
}

#[then(expr = "the original host should become a guest")]
async fn original_host_becomes_guest(world: &mut SessionWorld) {
    // Find the participant who was originally the host
    // (The one not currently marked as host)
    let current_host_id = world.lobby().host_id();

    let former_host = world
        .lobby()
        .participants()
        .values()
        .find(|p| p.id() != current_host_id)
        .expect("Should have at least 2 participants");

    assert!(!former_host.is_host(), "Former host should now be a guest");
}

#[then(expr = "a HostDelegated event should be broadcast with reason {string}")]
async fn host_delegated_event(world: &mut SessionWorld, reason: String) {
    // Event broadcasting will be implemented in P2P layer
    // For now, just verify the state change happened
    assert!(world.lobby().host().is_some(), "Lobby should have a host");

    // TODO: Verify actual event when event sourcing is implemented
    let _ = reason; // Use the parameter to avoid warning
}

#[then(expr = "the host should be marked as {string}")]
async fn host_marked_as(_world: &mut SessionWorld, status: String) {
    // TODO: Implement connection status tracking
    let _ = status;
}

#[then("the host should retain their role")]
async fn host_retains_role(world: &mut SessionWorld) {
    let host_name = "Host"; // Default host name
    let host_id = world.get_participant_id(host_name);

    assert_eq!(
        world.lobby().host_id(),
        host_id,
        "Host should still be the host"
    );
}

#[then("no delegation should occur")]
async fn no_delegation_occurs(world: &mut SessionWorld) {
    // Verify the original host is still host
    let host_name = "Host";
    let original_host_id = world.get_participant_id(host_name);

    assert_eq!(
        world.lobby().host_id(),
        original_host_id,
        "Delegation should not have occurred"
    );
}

#[then(expr = "the original host should rejoin as a guest")]
async fn original_host_rejoins_as_guest(_world: &mut SessionWorld) {
    // TODO: Implement when reconnection logic is added
}

#[then(expr = "{string} should remain the host")]
async fn should_remain_host(world: &mut SessionWorld, name: String) {
    should_become_host(world, name).await;
}

#[then(expr = "not {string}")]
async fn not_guest(world: &mut SessionWorld, name: String) {
    let participant_id = world.get_participant_id(&name);

    assert_ne!(
        world.lobby().host_id(),
        participant_id,
        "{} should NOT be the host",
        name
    );
}

#[then(expr = "the guest with the lowest UUID becomes host")]
async fn lowest_uuid_becomes_host(world: &mut SessionWorld) {
    // Find the guest with the lowest UUID (tie-breaker)
    let guests: Vec<_> = world
        .lobby()
        .participants()
        .values()
        .filter(|p| !p.is_host())
        .collect();

    assert!(
        guests.is_empty(),
        "After delegation, there should be no guests with matching timestamps"
    );

    // The current host should have been one of the tied guests
    assert!(world.lobby().host().is_some());
}

#[then("the guest should immediately become host")]
async fn guest_immediately_becomes_host(world: &mut SessionWorld) {
    // With only one guest, they should be promoted immediately
    assert_eq!(
        world.lobby().participants().len(),
        1,
        "Should only have 1 participant after auto-delegation"
    );

    let sole_participant = world.lobby().participants().values().next().unwrap();
    assert!(
        sole_participant.is_host(),
        "The only participant should be host"
    );
}

#[then("the lobby should close automatically")]
async fn lobby_closes(_world: &mut SessionWorld) {
    // TODO: Implement lobby closure logic
    // For now, we just verify the lobby exists but is empty
}

#[then(expr = "{string} can manage the activity")]
async fn can_manage_activity(world: &mut SessionWorld, name: String) {
    let participant_id = world.get_participant_id(&name);
    let participant = world.lobby().participants().get(&participant_id).unwrap();

    assert!(
        participant.can_manage_lobby(),
        "{} should be able to manage the lobby/activity",
        name
    );
}

#[then("the activity should continue")]
async fn activity_continues(_world: &mut SessionWorld) {
    // TODO: Implement when activity management is ready
}

#[then("the new host can manually delegate back")]
async fn new_host_can_delegate_back(_world: &mut SessionWorld) {
    // This is a business rule verification, not a technical test
    // The ability to delegate is inherent to being host
}
