Feature: Rating Match Protocol
  As designed in konnekt-session-workspace/protocols/rating-match.md

  Two players agree on a rated result without a server. Each player runs the
  same state machine. Every round is co-signed when it ends, then the match
  record is co-signed. These scenarios drive one player's machine with the
  protocol's event names.

  Background:
    Given a rating match with 3 rounds

  Scenario: A match starts by exchanging rating snapshots
    Then the rating match should be in state "ExchangingSnapshots"
    And the rating match should not be finished

  Scenario: Every round and the match are co-signed
    When both rating snapshots are verified
    Then the rating match should be in state "Playing"
    When 2 rounds are played and co-signed
    Then the rating match should be in state "Playing"
    When 1 round is played and co-signed
    Then the rating match should be in state "SigningMatch"
    And 3 rounds should be co-signed
    When "match_signed" happens
    Then the rating match should be in state "Rated"
    And the rating match should be finished

  Scenario: A one-round match goes straight to match signing
    Given a rating match with 1 round
    And both rating snapshots are verified
    When 1 round is played and co-signed
    Then the rating match should be in state "SigningMatch"

  Scenario Outline: Snapshot problems abort the match
    When "<event>" happens
    Then the rating match should be in state "Aborted"
    And the rating match should be finished

    Examples:
      | event             |
      | snapshot_rejected |
      | snapshot_timeout  |

  Scenario: Different round records end in disagreement
    Given both rating snapshots are verified
    When "round_finished" happens
    And "round_mismatch" happens
    Then the rating match should be in state "Disagreement"

  Scenario: Different match records end in disagreement
    Given both rating snapshots are verified
    And 3 rounds are played and co-signed
    When "match_mismatch" happens
    Then the rating match should be in state "Disagreement"

  Scenario: A rejoin while playing returns to playing
    Given both rating snapshots are verified
    And 1 round is played and co-signed
    When "opponent_left" happens
    Then the rating match should be in state "WaitingForOpponent"
    When "opponent_rejoined" happens
    Then the rating match should be in state "Playing"
    And 1 round should be co-signed

  Scenario Outline: A rejoin while signing a round returns to round signing
    Given both rating snapshots are verified
    And 1 round is played and co-signed
    And "round_finished" happens
    When "<leave>" happens
    Then the rating match should be in state "WaitingForOpponent"
    When "opponent_rejoined" happens
    Then the rating match should be in state "SigningRound"

    Examples:
      | leave             |
      | opponent_left     |
      | signature_timeout |

  Scenario: A rejoin during match signing returns to match signing
    Given both rating snapshots are verified
    And 3 rounds are played and co-signed
    When "signature_timeout" happens
    And "opponent_rejoined" happens
    Then the rating match should be in state "SigningMatch"

  Scenario: The opponent leaves for good after a co-signed round
    Given both rating snapshots are verified
    And 1 round is played and co-signed
    When "opponent_left" happens
    And "rejoin_timeout" happens
    Then the rating match should be in state "Forfeited"
    And the rating match should be finished

  Scenario: The opponent leaves for good before any co-signed round
    Given both rating snapshots are verified
    When "opponent_left" happens
    And "rejoin_timeout" happens
    Then the rating match should be in state "Aborted"

  Scenario: Events that don't fit the state are rejected
    Then "match_signed" should be rejected
    And "round_finished" should be rejected
    And "opponent_left" should be rejected
    When both rating snapshots are verified
    Then "round_signed" should be rejected
    And "opponent_rejoined" should be rejected
    And "rejoin_timeout" should be rejected
    And the rating match should be in state "Playing"

  Scenario Outline: A finished match accepts no more events
    Given both rating snapshots are verified
    And 3 rounds are played and co-signed
    And "<end>" happens
    Then "opponent_left" should be rejected
    And "match_signed" should be rejected
    And the rating match should be in state "<final>"

    Examples:
      | end            | final        |
      | match_signed   | Rated        |
      | match_mismatch | Disagreement |
