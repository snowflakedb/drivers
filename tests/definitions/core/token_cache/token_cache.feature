@core
Feature: SSO/MFA Token caching

  # --- file_cache.rs: hash_cache_key_tests ---

  @core_unit
  Scenario: Should produce deterministic SHA256
    Given a cache key string
    When we hash it multiple times
    Then each result should be identical and 64 hex characters long

  @core_unit
  Scenario: Should produce different hashes for different keys
    Given two distinct cache key strings
    When we hash each
    Then the hashes should differ

  # --- file_cache.rs: file_token_cache_tests ---

  @core_unit
  Scenario: Should set and get secret
    Given a file-based credential store
    When we set a secret for a key
    Then we should be able to retrieve the same secret

  @core_unit
  Scenario: Should return none for missing key
    Given a file-based credential store with no entries
    When we get a secret for a nonexistent key
    Then None should be returned

  @core_unit
  Scenario: Should delete existing credential
    Given a file-based credential store with a stored secret
    When we delete the credential
    Then the key should no longer exist

  @core_unit
  Scenario: Should return false when deleting nonexistent key
    Given a file-based credential store with no entries
    When we delete a nonexistent key
    Then the result should indicate the key did not exist

  @core_unit
  Scenario: Should overwrite secret
    Given a file-based credential store with a stored secret
    When we set a new secret for the same key
    Then the new secret should replace the old one

  @core_unit
  Scenario: Should store different keys separately
    Given a file-based credential store
    When we set secrets for two different keys
    Then each key should return its own secret independently

  @core_unit
  Scenario: Should use correct cache file name
    Given a file-based credential store
    When a secret is stored
    Then the backing file should be named credential_cache_v2.json

  @core_unit
  Scenario: Should contain valid JSON in cache file
    Given a file-based credential store with a stored secret
    When we read the backing file
    Then it should contain valid JSON with a "tokens" key

  @core_unit
  Scenario: Should SHA256 hash keys in file
    Given a file-based credential store with a stored secret
    When we inspect the backing JSON file
    Then the key should be the SHA-256 hash of the original key

  @core_unit
  Scenario: Should set cache file mode to 600
    Given a file-based credential store on a Unix system
    When a secret is stored
    Then the cache file permissions should be 0600

  @core_unit
  Scenario: Should remediate file with wrong permissions
    Given a cache file with permissions other than 0600
    When we read a secret
    Then permissions should be fixed to 0600 and the correct secret returned

  @core_unit
  Scenario: Should accept file owned by current user
    Given a cache file created by the current user
    When we read a secret
    Then the ownership check should pass

  @core_unit
  Scenario: Should reject file not owned by current user
    Given a cache file owned by a different user
    When we attempt to read a secret
    Then a FileNotOwnedByCurrentUser error should be returned

  @core_unit
  Scenario: Should remove lock directory after operation
    Given a file-based credential store
    When an operation completes
    Then the .lck directory should not exist

  @core_unit
  Scenario: Should break stale lock
    Given a stale lock directory older than the configured timeout
    When we perform an operation on the cache
    Then the stale lock should be broken and the operation should succeed

  @core_unit
  Scenario: Should support configurable retry parameters
    Given a file-based credential store with custom retry settings
    Then the retry count, delay, and stale lock timeout should match

  # --- file_cache.rs: concurrency_tests ---

  @core_unit
  Scenario: Should not corrupt data under concurrent writes
    Given a shared file-based credential store
    When multiple tasks write different keys simultaneously
    Then every key should hold the correct value

  @core_unit
  Scenario: Should return consistent data during concurrent reads and writes
    Given a shared file-based credential store with pre-seeded keys
    When readers and writers operate simultaneously on the same keys
    Then every read should return either the old or new value, never corrupt data

  @core_unit
  Scenario: Should remain consistent under concurrent deletes
    Given a shared file-based credential store with multiple keys
    When half the keys are deleted concurrently
    Then deleted keys should be gone and untouched keys should remain

  # --- file_cache.rs: file_credential_adapter_tests ---

  @core_unit
  Scenario: Should set and get password via credential adapter
    Given a file-based credential builder and a credential
    When we set a password and then get it
    Then the retrieved password should match

  @core_unit
  Scenario: Should return no entry for missing credential adapter password
    Given a file-based credential builder and a credential with no stored value
    When we get the password
    Then a NoEntry error should be returned

  @core_unit
  Scenario: Should delete existing credential via adapter
    Given a credential with a stored password
    When we delete the credential
    Then getting the password should return NoEntry

  @core_unit
  Scenario: Should return no entry when deleting missing credential via adapter
    Given a credential with no stored value
    When we delete the credential
    Then a NoEntry error should be returned

  @core_unit
  Scenario: Should overwrite password via credential adapter
    Given a credential with a stored password
    When we set a new password
    Then the new password should replace the old one

  @core_unit
  Scenario: Should keep separate credentials independent via adapter
    Given two credentials with different keys from the same builder
    When we set different passwords on each
    Then each credential should return its own password

  @core_unit
  Scenario: Should report persistence as until delete via adapter
    Given a file-based credential builder
    Then its persistence should be UntilDelete

  @core_unit
  Scenario: Should share same backing file across credentials
    Given two credential instances for the same key
    When one sets a password
    Then the other should be able to read it

  # --- mod.rs: keyring_token_cache_tests ---

  @core_unit
  Scenario: Should add and get token via keyring singleton
    Given the keyring token cache singleton
    When we add a token and then retrieve it
    Then the retrieved token should match

  @core_unit
  Scenario: Should return none for nonexistent keyring token
    Given the keyring token cache singleton with no stored token
    When we get a token
    Then None should be returned

  @core_unit
  Scenario: Should remove existing token from keyring
    Given the keyring token cache singleton with a stored token
    When we remove the token
    Then getting it should return None

  @core_unit
  Scenario: Should succeed when removing nonexistent keyring token
    Given the keyring token cache singleton with no stored token
    When we remove a token
    Then the operation should succeed

  @core_unit
  Scenario: Should overwrite token in keyring
    Given the keyring token cache singleton with a stored token
    When we add a new value for the same key
    Then the new value should replace the old one

  @core_unit
  Scenario: Should store different token types separately in keyring
    Given the keyring token cache singleton
    When we store tokens of different types for the same host and user
    Then each type should return its own value

  @core_unit
  Scenario: Should fail to add keyring token with empty host
    Given the keyring token cache singleton
    When we add a token with an empty host
    Then an InvalidKeyFormat error should be returned

  @core_unit
  Scenario: Should fail to add keyring token with empty username
    Given the keyring token cache singleton
    When we add a token with an empty username
    Then an InvalidKeyFormat error should be returned

  @core_unit
  Scenario: Should fail to get keyring token with empty host
    Given the keyring token cache singleton
    When we get a token with an empty host
    Then an InvalidKeyFormat error should be returned

  @core_unit
  Scenario: Should fail to get keyring token with empty username
    Given the keyring token cache singleton
    When we get a token with an empty username
    Then an InvalidKeyFormat error should be returned

  @core_unit
  Scenario: Should fail to remove keyring token with empty host
    Given the keyring token cache singleton
    When we remove a token with an empty host
    Then an InvalidKeyFormat error should be returned

  @core_unit
  Scenario: Should fail to remove keyring token with empty username
    Given the keyring token cache singleton
    When we remove a token with an empty username
    Then an InvalidKeyFormat error should be returned
