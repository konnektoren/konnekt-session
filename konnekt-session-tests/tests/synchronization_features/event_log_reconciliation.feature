Feature: Comparing session histories

  Scenario: A guest compares matching verified histories
    Given a guest has synchronized with the host
    And the guest begins reconciling with the connected host
    When the host authority is verified and both histories match
    Then the local reconciliation reports matching histories

  Scenario: A disconnected peer cancels reconciliation
    Given a guest has synchronized with the host
    And the guest begins reconciling with the connected host
    When the reconciliation peer disconnects
    Then the local reconciliation is cancelled

  Scenario: A host shares an offered history with a guest
    Given a guest has synchronized with the host
    And the guest begins reconciling with the connected host
    When the host shares its history with the guest
    Then the guest receives the offered history for verification

  Scenario: A guest ignores an unsigned history reply
    Given a guest has synchronized with the host
    And the guest begins reconciling with the connected host
    When the guest receives an unsigned history reply
    Then the guest ignores the unsigned history reply

  Scenario: A guest rejects an unexpected host key
    Given a guest has synchronized with the host
    And the guest begins reconciling with the connected host
    When the guest requests history using an unexpected host key
    Then the guest does not receive a history

  Scenario: An oversized history is not shared
    Given a guest has synchronized with the host
    And the guest begins reconciling with the connected host
    When the host tries to offer more than 100 history events
    Then no history is offered to the guest
