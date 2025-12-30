Feature: Peer-Participant Mapping (Infrastructure)
  Internal P2P layer mapping between network peers and domain participants.
  This is an infrastructure concern, not a business feature.

  @infrastructure
  Scenario: Map peer to participant
    Given an empty peer-participant mapping
    And a peer with ID "peer-1"
    And a participant with ID "participant-1"
    When I register peer "peer-1" to participant "participant-1"
    Then the mapping should contain 1 entry
    And peer "peer-1" should map to participant "participant-1"

  @infrastructure
  Scenario: Remove mapping by peer
    Given peer "peer-1" is mapped to participant "participant-1"
    When I remove the mapping for peer "peer-1"
    Then the mapping should be empty
    And peer "peer-1" should not be mapped

  @infrastructure
  Scenario: Query participant by peer
    Given peer "peer-1" is mapped to participant "participant-1"
    When I query participant for peer "peer-1"
    Then participant "participant-1" should map to peer "peer-1"

  @infrastructure
  Scenario: Clear all mappings
    Given peer "peer-1" is mapped to participant "participant-1"
    And peer "peer-2" is mapped to participant "participant-2"
    When I clear all mappings
    Then the mapping should be empty
