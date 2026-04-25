Feature: Bevy Application Layer
  As a consuming app
  I want to drive session state via Bevy app ticks
  So that command processing is deterministic and testable

  Scenario: Create lobby through Bevy command bus
    Given a Bevy session app is initialized
    When the host submits CreateLobby for "Bevy Lobby"
    And the Bevy app ticks 1 time
    Then the Bevy event log should contain 1 event
    And event 1 should be "LobbyCreated" with sequence 1
    And the lobby "Bevy Lobby" should have 1 participants in Bevy domain

  Scenario: Join lobby through Bevy command bus
    Given a Bevy session app is initialized
    When the host submits CreateLobby for "Bevy Lobby"
    And a guest named "Alice" submits JoinLobby
    And the Bevy app ticks 1 time
    Then the Bevy event log should contain 2 event
    And event 1 should be "LobbyCreated" with sequence 1
    And event 2 should be "GuestJoined" with sequence 2
    And the lobby "Bevy Lobby" should have 2 participants in Bevy domain
