Feature: Sharing a game asset

  Scenario: Players complete a transfer despite reordered and repeated chunks
    Given a player has an asset to share
    When the asset arrives out of order with a repeated chunk
    Then both players complete the transfer with the original asset

  Scenario: A cancelled transfer cannot complete
    Given a player has an asset to share
    When the receiving player cancels the transfer
    Then later chunks cannot complete the asset
