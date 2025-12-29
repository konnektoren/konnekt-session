Feature: Lobby Management
  As documented in docs/02-discover.adoc - Lobby Management subdomain

  The lobby is the aggregate root containing participants and activities.
  Host/Guest model with clear authority hierarchy.

  Scenario: Create a new lobby
    Given a user wants to create a lobby
    When they create a lobby named "Friday Night Quiz" with password "fun123"
    Then a lobby should be created
    And the lobby should have a unique ID
    And the creator should be the host
    And the lobby status should be "Open"

  Scenario: Join lobby with correct password
    Given a lobby exists with password "secret123"
    When a guest joins with the correct password
    Then the guest should be added to the lobby
    And the guest should be in Active mode
    And a GuestJoined event should be broadcast

  Scenario: Join lobby with wrong password
    Given a lobby exists with password "secret123"
    When a guest tries to join with password "wrong123"
    Then the join should be rejected
    And the error should be "Invalid password"

  Scenario: Join lobby at capacity
    Given a lobby exists with max 3 guests
    And 3 guests have already joined
    When another guest tries to join
    Then the join should be rejected
    And the error should be "Lobby full"

  Scenario: Close lobby
    Given a lobby exists with 2 guests
    And the host decides to close the lobby
    When the host closes the lobby
    Then the lobby status should be "Closed"
    And new guests cannot join
    But existing activities can continue

  Scenario: Archive lobby after 24 hours
    Given a lobby has been closed
    And no events occurred for 24 hours
    When the archive policy runs
    Then the lobby status should be "Archived"
    And the lobby should be read-only

  Scenario: Lobby with unique names
    Given a lobby exists
    When a guest tries to join with a duplicate name
    Then the join should be rejected
    And the error should be "Name already taken"

  Scenario: Lobby capacity validation
    Given a user wants to create a lobby
    When they set max guests to 15
    Then the creation should be rejected
    And the error should be "Max guests cannot exceed 10"
