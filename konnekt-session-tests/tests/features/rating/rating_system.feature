Feature: Rating Systems
  As designed in konnekt-session-rating

  Glicko-2 is the default rating system, Elo is available.
  New players start at 1000. Client-side ratings are capped at 1500.
  A weight scales the change: 1.0 for a normal match, 0.25 for a ghost game.

  Background:
    Given rated players "Alice" and "Bob"

  Scenario: New players start at 1000
    Then "Alice" should have a rating of 1000
    And "Bob" should have a rating of 1000

  Scenario: Winner gains, loser loses
    When "Alice" wins against "Bob"
    And the match is rated
    Then the rating of "Alice" should have increased
    And the rating of "Bob" should have decreased

  Scenario: Draw between equal players keeps the rating
    When "Alice" and "Bob" draw
    And the match is rated
    Then "Alice" should have a rating of 1000
    And "Bob" should have a rating of 1000

  Scenario Outline: Elo change scaled by weight
    Given the Elo rating system
    When "Alice" wins against "Bob"
    And the match is rated with weight <weight>
    Then "Alice" should have a rating of <alice>
    And "Bob" should have a rating of <bob>

    Examples:
      | weight | alice | bob  |
      | 1.0    | 1016  | 984  |
      | 0.25   | 1004  | 996  |
      | 0.0    | 1000  | 1000 |

  Scenario: Client ratings are capped
    Given "Alice" has a rating of 1499
    And "Bob" has a rating of 1499
    When "Alice" wins against "Bob"
    And the match is rated
    Then "Alice" should have a rating of 1500

  Scenario: Every placed player needs a rating
    Given player "Dave" without a rating
    When "Alice" wins against "Dave"
    And the match is rated
    Then rating should be rejected with "No rating given"

  Scenario: Weight must be between 0 and 1
    When "Alice" wins against "Bob"
    And the match is rated with weight 1.5
    Then rating should be rejected with "Weight must be within"
