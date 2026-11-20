Feature: Rated Game
  As designed in konnekt-session-rating

  Games stay in the consuming app and implement RatedGame on core's
  ActivityRun. The test game "High Score" ranks players by score.

  Background:
    Given a lobby exists with Host "Alice"
    And guest "Bob" has joined
    And rated players "Alice" and "Bob"
    And a rated "High Score" activity is in progress

  Scenario: Completed run is rated by score
    When "Alice" submits score 80 for the rated activity
    And "Bob" submits score 60 for the rated activity
    Then the rated game should place "Alice" before "Bob"
    And the rating key should be "test.high-score.v1"

  Scenario: Equal scores are a draw
    When "Alice" submits score 70 for the rated activity
    And "Bob" submits score 70 for the rated activity
    Then the rated game should place "Alice" and "Bob" equal

  Scenario: Incomplete run is not rated
    When "Alice" submits score 80 for the rated activity
    Then the run should not be rated

  Scenario: Rating after a completed run
    When "Alice" submits score 60 for the rated activity
    And "Bob" submits score 90 for the rated activity
    And the run is rated
    Then the rating of "Bob" should have increased
    And the rating of "Alice" should have decreased
