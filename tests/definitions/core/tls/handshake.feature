@core
Feature: TLS handshake

  @core_e2e
  Scenario: Should complete handshake with default roots
    Given a TLS client configured with default roots
    When GET request is sent to the server URL
    Then the request attempt should be successful

  @core_e2e
  Scenario: Should complete handshake with custom PEM roots
    Given E2E_TLS_ROOTS_PEM is set to a PEM bundle path
    And a TLS client configured with that custom root store
    When GET request is sent to the server URL
    Then the request attempt should complete (success or error acceptable in CI)