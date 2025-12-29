Feature: End-to-End Integration Flows
  Complete user journeys combining multiple features

  Scenario: Complete game session
    Given Alice creates a lobby "Friday Quiz" with password "fun123"
    When Bob joins with the correct password
    And Carol joins with the correct password
    And Alice plans a "Trivia Round 1" activity
    And Alice starts the activity
    And Bob submits (score 8)
    And Carol submits (score 7)
    And Alice submits (score 9)
    Then the activity completes
    And the leaderboard shows Alice: 9, Bob: 8, Carol: 7
    When Alice plans a "Trivia Round 2" activity
    And Carol toggles to Spectating
    And Alice starts the activity
    And Bob submits (score 6)
    And Alice submits (score 10)
    Then the activity completes
    And the leaderboard shows Alice: 10, Bob: 6
    And Carol is not ranked

  Scenario: Host handover mid-session
    Given Alice creates a lobby
    And Bob joins
    And Carol joins
    And an activity is in progress
    When Alice needs to leave
    And Alice delegates host to Bob
    Then Bob becomes the host
    And can continue managing the lobby
    When Alice leaves
    And the activity completes
    Then Bob can start the next activity

  Scenario: Network resilience
    Given a lobby with 3 participants
    And an activity is in progress
    When the host disconnects temporarily
    And reconnects within 30 seconds
    Then the host retains their role
    And the activity continues
    When a guest disconnects for 35 seconds
    Then the guest is removed
    But the activity can complete with remaining participants
