Feature: Verifying session history

  Scenario: A trusted signed history can be verified
    Given a session has a signed event history
    When the signed history is verified against the trusted author
    Then the history is verified through event 2

  Scenario: A changed event is rejected
    Given a session has a signed event history
    When a signed event is changed before verification
    Then verification fails with "event signature is invalid"

  Scenario: An unknown author is rejected
    Given a session has a signed event history
    When the signed history is checked without trusting its author
    Then verification fails with "event author is not trusted"

  Scenario: Missing ancestry needs a trusted checkpoint
    Given a session has a signed event history
    When only the later event is available without a checkpoint
    Then verification fails with "history does not start at genesis and has no trusted anchor"

  Scenario: A host retains a verifiable checkpoint when older events are evicted
    Given a host records signed history with room for two events
    When the host queues two activities
    Then the retained signed events verify using the evicted checkpoint

  Scenario: A guest receives a signed history and verifies a trusted checkpoint
    Given a host and guest connect while the host records signed history
    When the host shares its retained signed history
    Then the guest verifies the signed history using its trusted checkpoint
