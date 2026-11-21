Feature: Session synchronization
  A guest can see when its view has a gap in host updates.

  Scenario: A guest waits for a missing update
    Given a guest has synchronized with the host
    When a later host update arrives before an earlier update
    Then the guest is recovering its session

  Scenario: A guest becomes current when the missing update arrives
    Given a guest has synchronized with the host
    And a later host update arrived before an earlier update
    When the missing host update arrives
    Then the guest is current again
