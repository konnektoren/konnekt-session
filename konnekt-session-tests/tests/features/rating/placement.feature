Feature: Match Placement
  As designed in konnekt-session-rating

  Games report who placed how. A placement lists groups of tied
  players, best first: a win is [[a], [b]], a draw is [[a, b]].

  Background:
    Given rated players "Alice", "Bob" and "Carol"

  Scenario: Win
    When "Alice" wins against "Bob"
    Then "Alice" should be on place 1
    And "Bob" should be on place 2

  Scenario: Draw
    When "Alice" and "Bob" draw
    Then "Alice" should be on place 1
    And "Bob" should be on place 1

  Scenario: Placement from scores with a tie
    When the placement is made from the scores:
      | player | score |
      | Alice  | 50    |
      | Bob    | 90    |
      | Carol  | 50    |
    Then "Bob" should be on place 1
    And "Alice" should be on place 2
    And "Carol" should be on place 2

  Scenario: Player not in the match
    When "Alice" wins against "Bob"
    Then "Carol" should not be placed

  Scenario: A player cannot be placed twice
    When "Alice" wins against "Alice"
    Then the placement should be rejected with "appears more than once"

  Scenario: A match needs at least two players
    When the placement is made from the scores:
      | player | score |
      | Alice  | 50    |
    Then the placement should be rejected with "at least two players"
