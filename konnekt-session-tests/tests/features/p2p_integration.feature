Feature: P2P and Core Domain Integration
  As documented in docs/03-decompose.adoc - Anti-Corruption Layer

  The P2P layer automatically translates between network events and domain commands.
  This maintains separation of concerns while enabling bidirectional communication.

  Background:
    Given a P2P session is initialized

  Scenario: Core event is broadcast via P2P
    Given the core domain emits a GuestJoined event
    When the P2P loop processes the event
    Then the event should be broadcast to all peers
    And the event should have a sequence number assigned

  Scenario: P2P event is translated to domain command
    Given a GuestJoined event is received from P2P
    When the P2P loop polls
    Then a JoinLobby command should be queued
    And the command should have the correct lobby ID

  Scenario: Roundtrip translation preserves data
    Given a core GuestJoined event for "Alice"
    When the event is translated to P2P and back to command
    Then the resulting command should contain "Alice"

  Scenario: CommandFailed is not broadcast
    Given the core domain emits a CommandFailed event
    When the P2P loop processes the event
    Then no P2P event should be broadcast

  Scenario: Host delegation flows through P2P
    Given a core HostDelegated event
    When the event is broadcast via P2P
    Then peers should receive HostDelegated
    And peers should translate to DelegateHost command

  Scenario: Participation mode changes sync via P2P
    Given a core ParticipationModeChanged event
    When the event is broadcast via P2P
    Then peers should receive ParticipationModeChanged
    And peers should translate to ToggleParticipationMode command
