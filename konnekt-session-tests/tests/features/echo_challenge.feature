Feature: Echo Challenge Activity
  The simplest activity for testing - participants echo back a prompt.
  Score = 100 if exact match, 0 otherwise.

  Background:
    Given a lobby exists with a host
    And guest "Alice" in Active mode
    And guest "Bob" in Active mode

  Scenario: Plan an Echo Challenge
    When the host plans an Echo Challenge with prompt "Hello Rust"
    Then the activity should be in Planned status
    And the activity type should be "echo-challenge-v1"
    And the activity name should be "Echo: Hello Rust"

  Scenario: Start Echo Challenge
    Given an Echo Challenge with prompt "WebAssembly" is planned
    When the host starts the activity
    Then the activity status should be InProgress

  Scenario: Submit correct answer
    Given an Echo Challenge with prompt "Konnekt" is in progress
    When "Alice" submits response "Konnekt"
    Then "Alice" should receive score 100
    And the result should be recorded

  Scenario: Submit incorrect answer (case sensitive)
    Given an Echo Challenge with prompt "Rust" is in progress
    When "Alice" submits response "rust"
    Then "Alice" should receive score 0
    And the result should be recorded

  Scenario: Submit incorrect answer (partial match)
    Given an Echo Challenge with prompt "Hello World" is in progress
    When "Alice" submits response "Hello"
    Then "Alice" should receive score 0
    And the result should be recorded

  Scenario: Activity completes when all active participants submit
    Given an Echo Challenge with prompt "Test" is in progress
    When the host submits response "Test" (score 100)
    And "Alice" submits response "Test" (score 100)
    And "Bob" submits response "test" (score 0)
    Then the activity should complete
    And 3 results should be recorded

  Scenario: Spectator cannot submit Echo response
    Given guest "Carol" in Spectating mode
    And an Echo Challenge with prompt "Echo" is in progress
    When "Carol" tries to submit response "Echo"
    Then the submission should be rejected
    And the error should be "Spectators cannot submit results"

  Scenario: Time tracking for Echo responses
    Given an Echo Challenge with prompt "Speed" is in progress
    When "Alice" submits response "Speed" after 1500 milliseconds
    Then the result should record time 1500 milliseconds
    And "Alice" should receive score 100

  Scenario: Echo Challenge with time limit (validation)
    Given an Echo Challenge with prompt "Fast" and time limit 5000ms is planned
    When the host starts the activity
    And "Alice" submits response "Fast" after 3000 milliseconds
    Then the result should be accepted
    And "Alice" should receive score 100

  Scenario: Multiple Echo Challenges in sequence
    Given an Echo Challenge with prompt "First" is completed
    When the host plans an Echo Challenge with prompt "Second"
    And the host starts the activity
    And "Alice" submits response "Second"
    Then the new activity should complete
    And results from "First" should be preserved

  Scenario: Serialization of Echo Challenge
    Given an Echo Challenge with prompt "Serialize Test"
    When the activity config is serialized to JSON
    And deserialized back to an Echo Challenge
    Then the prompt should be "Serialize Test"

  Scenario: Empty prompt is allowed (edge case)
    When the host plans an Echo Challenge with prompt ""
    Then the activity should be created
    And the prompt should be ""
