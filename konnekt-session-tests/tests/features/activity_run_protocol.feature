Feature: Activity run lifecycle

  Scenario: Cancelling a run keeps submitted results
    Given a lobby exists with a host
    And guest "Alice" joined 0 seconds ago
    And an Echo Challenge with prompt "Test" is in progress
    And "Alice" has submitted response "Test"
    When the host cancels the current Echo Challenge
    Then the Echo Challenge is cancelled with Alice's result
