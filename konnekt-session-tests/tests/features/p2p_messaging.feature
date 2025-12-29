Feature: P2P Messaging and State Sync
  As documented in docs/05-connect.adoc - Domain Message Flows

  All P2P messages are signed and verified.
  Round-robin state sync with message ordering.

  Background:
    Given a lobby exists with a host
    And 2 guests have joined

  Scenario: Message signature verification
    Given a guest creates a message
    When the message is signed with their private key
    Then other participants can verify the signature

  Scenario: Reject message with invalid signature
    Given a guest creates a message
    When the message has an invalid signature
    Then the message should be rejected
    And a security error should be logged

  Scenario: Stale message rejection (replay attack prevention)
    Given a message was created 65 seconds ago
    When participants try to process the message
    Then the message should be rejected
    And the error should be "Stale message"

  Scenario: Message sequence ordering
    Given participant sends messages with seq [1, 2, 3]
    When message 3 arrives before message 2
    Then message 3 should be queued
    When message 2 arrives
    Then messages should be processed in order [2, 3]

  Scenario: Detect missing messages
    Given participant sends messages with seq [1, 2, 4]
    When message 4 arrives
    Then a gap should be detected
    And a request for message 3 should be sent

  Scenario: Heartbeat monitoring
    Given all participants are connected
    When a participant sends a heartbeat every 5 seconds
    Then the last heartbeat time should be updated

  Scenario: Disconnect detection after 10s
    Given the host's last heartbeat was 10 seconds ago
    When participants check connectivity
    Then the host should be marked as "suspected disconnect"

  Scenario: Disconnect confirmation after 30s
    Given the host's last heartbeat was 30 seconds ago
    When participants check connectivity
    Then the host should be marked as "confirmed disconnect"
    And host delegation should trigger

  Scenario: Broadcast to all peers
    Given 3 participants in the lobby
    When the host broadcasts a LobbyStateUpdate
    Then all 3 participants should receive it

  Scenario: State reconciliation on reconnect
    Given a guest disconnects
    And the lobby state changes (new guest joined)
    When the guest reconnects
    Then the guest should receive the current LobbyState
    And their local state should sync

  Scenario: Round-robin leader rotation
    Given 3 participants in the lobby
    When cycle 1 completes
    Then participant A broadcasts
    When cycle 2 completes
    Then participant B broadcasts
    When cycle 3 completes
    Then participant C broadcasts

  Scenario: Conflicting state updates (last-write-wins)
    Given two participants update the same state
    When both updates are broadcast
    Then the update with the latest timestamp wins
    And other participants apply the winning update
