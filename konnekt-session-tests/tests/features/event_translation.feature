Feature: Event Translation Between P2P and Core Domains
  As documented in DDD architecture, the EventTranslator is an Anti-Corruption Layer (ACL)
  that keeps the P2P and Core bounded contexts separate.

  Background:
    Given a lobby with ID "lobby-123"
    And an event translator for lobby "lobby-123"
  # P2P → Core Command Translation

  Scenario: Translate GuestJoined to JoinLobby command
    Given a P2P event "GuestJoined" for participant "Alice"
    When I translate the P2P event to a domain command
    Then the command should be "JoinLobby"
    And the command lobby ID should be "lobby-123"
    And the command should contain guest name "Alice"

  Scenario: Translate GuestLeft to LeaveLobby command
    Given a P2P event "GuestLeft" for participant "participant-456"
    When I translate the P2P event to a domain command
    Then the command should be "LeaveLobby"
    And the command lobby ID should be "lobby-123"
    And the command should contain participant ID "participant-456"

  Scenario: Translate GuestKicked to KickGuest command
    Given a P2P event "GuestKicked" with guest "guest-1" kicked by "host-1"
    When I translate the P2P event to a domain command
    Then the command should be "KickGuest"
    And the command lobby ID should be "lobby-123"
    And the command should contain guest ID "guest-1"
    And the command should contain host ID "host-1"

  Scenario: Translate HostDelegated to DelegateHost command
    Given a P2P event "HostDelegated" from "host-1" to "guest-2"
    When I translate the P2P event to a domain command
    Then the command should be "DelegateHost"
    And the command lobby ID should be "lobby-123"
    And the command should contain current host "host-1"
    And the command should contain new host "guest-2"

  Scenario: Translate ParticipationModeChanged to ToggleParticipationMode command
    Given a P2P event "ParticipationModeChanged" for participant "participant-789"
    When I translate the P2P event to a domain command
    Then the command should be "ToggleParticipationMode"
    And the command lobby ID should be "lobby-123"
    And the command should contain participant ID "participant-789"

  Scenario: LobbyCreated event does not produce a command
    Given a P2P event "LobbyCreated" for lobby "lobby-123"
    When I translate the P2P event to a domain command
    Then the translation should return None
  # Core → P2P Event Translation

  Scenario: Translate LobbyCreated to P2P event
    Given a core event "LobbyCreated" for lobby "Test Lobby"
    When I translate the core event to a P2P event
    Then the P2P event should be "LobbyCreated"
    And the P2P event should contain lobby ID "lobby-123"
    And the P2P event should contain lobby name "Test Lobby"

  Scenario: Translate GuestJoined to P2P event
    Given a core event "GuestJoined" for participant "Bob"
    When I translate the core event to a P2P event
    Then the P2P event should be "GuestJoined"
    And the P2P event should contain participant name "Bob"

  Scenario: Translate GuestLeft to P2P event
    Given a core event "GuestLeft" for participant "participant-111"
    When I translate the core event to a P2P event
    Then the P2P event should be "GuestLeft"
    And the P2P event should contain participant ID "participant-111"

  Scenario: Translate GuestKicked to P2P event
    Given a core event "GuestKicked" with guest "guest-2" kicked by "host-1"
    When I translate the core event to a P2P event
    Then the P2P event should be "GuestKicked"
    And the P2P event should contain guest ID "guest-2"
    And the P2P event should contain kicked by "host-1"

  Scenario: Translate HostDelegated to P2P event
    Given a core event "HostDelegated" from "host-1" to "guest-3"
    When I translate the core event to a P2P event
    Then the P2P event should be "HostDelegated"
    And the P2P event should contain from "host-1"
    And the P2P event should contain to "guest-3"

  Scenario: Translate ParticipationModeChanged to P2P event
    Given a core event "ParticipationModeChanged" for participant "participant-222" to "Spectating"
    When I translate the core event to a P2P event
    Then the P2P event should be "ParticipationModeChanged"
    And the P2P event should contain participant ID "participant-222"
    And the P2P event should contain mode "Spectating"

  Scenario: CommandFailed event does not produce a P2P event
    Given a core event "CommandFailed" with reason "Invalid state"
    When I translate the core event to a P2P event
    Then the translation should return None
  # Roundtrip Translation

  Scenario: Roundtrip - GuestJoined → Command → GuestJoined
    Given a P2P event "GuestJoined" for participant "Charlie"
    When I translate to a command and back to a P2P event
    Then the final P2P event should be "GuestJoined"
    And the participant name should be preserved as "Charlie"

  Scenario: Roundtrip - GuestLeft → Command → GuestLeft
    Given a P2P event "GuestLeft" for participant "participant-333"
    When I translate to a command and back to a P2P event
    Then the final P2P event should be "GuestLeft"
    And the participant ID should be preserved as "participant-333"
