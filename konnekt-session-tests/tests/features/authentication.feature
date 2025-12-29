Feature: Authentication and Identity
  As documented in docs/adr/0004-use-client-side-key-generation-for-identity.adoc

  Ed25519 key pairs for participant identity.
  Deterministic key generation from name+password.

  Scenario: Generate key pair from name and password
    Given a user provides name "Alice" and password "secret123"
    When the system generates a key pair
    Then a deterministic Ed25519 key pair should be created
    And the same name+password always produces the same keys

  Scenario: Sign a message
    Given a participant has generated their key pair
    When they create a JoinRequest message
    Then the message should be signed with their private key
    And include their public key as sender ID

  Scenario: Verify message signature
    Given a participant receives a signed message
    When they verify the signature using the sender's public key
    Then the signature should be valid

  Scenario: Reject tampered message
    Given a participant receives a signed message
    When the message payload has been tampered with
    Then signature verification should fail

  Scenario: Persistent identity across sessions
    Given a user joins with name "Alice" and password "secret123"
    And generates key pair K1
    When they leave and rejoin with same credentials
    And generate key pair K2
    Then K1 should equal K2
    And their participant ID should be recognized

  Scenario: Key export for backup
    Given a participant has generated their key pair
    When they export their private key
    Then they receive a backup string
    And can import it to recover their identity

  Scenario: Clock skew tolerance
    Given a message timestamp is 5 seconds in the future
    When the message is verified
    Then the timestamp should be accepted

  Scenario: Clock skew rejection
    Given a message timestamp is 65 seconds old
    When the message is verified
    Then the message should be rejected as stale
