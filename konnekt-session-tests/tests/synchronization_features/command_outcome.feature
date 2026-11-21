Feature: Host command outcome
  The host can distinguish a queued request from a domain decision.

  Scenario: A Queued Request Remains Pending
    Given a guest has synchronized with the host
    When the host queues an activity
    Then the host request is pending

  Scenario: The Host Sees a Rejected Start
    Given a guest has synchronized with the host
    When the host starts a session with no planned activity
    Then the host request is rejected with a reason

  Scenario: The Host Sees an Accepted Activity
    Given a guest has synchronized with the host
    When the host queues and processes an activity
    Then the host request is accepted
