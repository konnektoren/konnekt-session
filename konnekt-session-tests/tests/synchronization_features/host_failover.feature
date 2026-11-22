Feature: Observing host availability

  Scenario: A guest notices the host leaving while another peer remains
    Given a guest has synchronized with the host
    When the observed host disconnects while another peer remains
    Then the guest observes the host grace period without taking authority

  Scenario: A guest waits after the host grace period expires
    Given a guest has synchronized with a host with no grace delay
    When the observed host disconnects and the grace period expires
    Then the guest waits for a candidate without taking authority

  Scenario: The host returns during the grace period
    Given a guest has synchronized with the host
    And the observed host disconnected while another peer remains
    When the same host reconnects and resynchronizes
    Then the guest observes that the host is available again
