Feature: Who Am I identity resolution
  The Yew session context should resolve local identity reliably
  using participant ID + peer identity, even if local name is not tracked.

  Scenario: Host identity includes domain role and P2P role
    Given a lobby named "WhoAmI Lobby" with host "Alice"
    And guest "Bob" has joined that lobby
    When I resolve who am i for "Alice" as p2p role "Host" with peer id "peer-host-1"
    Then who am i should report participant id for "Alice"
    And who am i should report participant name "Alice"
    And who am i should report lobby role "Host"
    And who am i should report participation mode "Active"
    And who am i should report p2p role "Host"
    And who am i should report local peer id "peer-host-1"

  Scenario: Guest identity includes domain role and P2P role
    Given a lobby named "WhoAmI Lobby" with host "Alice"
    And guest "Bob" has joined that lobby
    When I resolve who am i for "Bob" as p2p role "Guest" with peer id "peer-guest-1"
    Then who am i should report participant id for "Bob"
    And who am i should report participant name "Bob"
    And who am i should report lobby role "Guest"
    And who am i should report participation mode "Active"
    And who am i should report p2p role "Guest"
    And who am i should report local peer id "peer-guest-1"
