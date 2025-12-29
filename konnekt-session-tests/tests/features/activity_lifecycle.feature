Feature: Activity Lifecycle
  As documented in docs/02-discover.adoc - Activity Management

  Activity state machine: Planned → InProgress → Completed/Cancelled
  Only active participants count for completion.

  Background:
    Given a lobby exists with a host
    And guest "Alice" in Active mode
    And guest "Bob" in Active mode
    And guest "Carol" in Spectating mode

  Scenario: Plan activity
    When the host plans a "Trivia Quiz" activity
    Then the activity should be in Planned status
    And an ActivityPlanned event should be broadcast

  Scenario: Start activity
    Given a "Trivia Quiz" activity is planned
    When the host starts the activity
    Then the activity status should be InProgress
    And an ActivityStarted event should be broadcast
    And the start time should be recorded

  Scenario: Only one activity in progress
    Given a "Trivia Quiz" activity is in progress
    When the host tries to start another activity
    Then the start should be rejected
    And the error should be "Only one activity can be in progress"

  Scenario: Collect results from active participants
    Given a "Trivia Quiz" activity is in progress
    When "Alice" submits a result with score 8
    And "Bob" submits a result with score 9
    Then 2 results should be recorded
    But the activity should still be in progress

  Scenario: Activity completion when all active participants submit
    Given a "Trivia Quiz" activity is in progress
    And the host has submitted (score 7)
    And "Alice" has submitted (score 8)
    When "Bob" submits (score 9)
    Then the activity should complete
    And an ActivityCompleted event should be broadcast
    And the completion time should be recorded
    And "Carol" should not be counted

  Scenario: Activity continues when spectator present
    Given a "Trivia Quiz" activity is in progress
    And "Carol" is spectating
    When all active participants submit
    Then the activity should complete

  Scenario: Cancel activity
    Given a "Trivia Quiz" activity is in progress
    When the host cancels the activity
    Then the activity status should be Cancelled
    And an ActivityCancelled event should be broadcast

  Scenario: Remove planned activity
    Given a "Trivia Quiz" activity is planned
    When the host removes the activity from queue
    Then the activity should be removed
    And an ActivityRemoved event should be broadcast

  Scenario: Guest abandons activity mid-game
    Given a "Trivia Quiz" activity is in progress
    And "Alice" toggles to Spectating mid-activity
    Then an ActivityAbandoned event is recorded for "Alice"
    And the activity can complete without "Alice"

  Scenario: Guest disconnects during activity
    Given a "Trivia Quiz" activity is in progress
    And "Alice" disconnects
    When 30 seconds pass
    Then "Alice" is removed from active participants
    And the activity can complete without "Alice"

  Scenario: Leaderboard update after completion
    Given a "Trivia Quiz" activity has completed
    When the system calculates scores
    Then a LeaderboardUpdated event should be broadcast
    And spectators should be excluded from rankings

  Scenario: Activity timeout
    Given a "Trivia Quiz" activity started 35 minutes ago
    When the timeout policy runs
    Then the activity should be auto-cancelled
    And partial results should be recorded
