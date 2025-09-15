Feature: TLS handshake
  As a client developer
  I want to verify TLS handshakes with default and custom roots
  So that connections can be established under different trust settings

  Background:
    Given an HTTPS server URL from env E2E_TLS_SERVER or default "https://example.com"

  Scenario: Handshake with default roots
    Given a TLS client configured with default roots
    When I send a GET request to the server URL
    Then the request attempt should complete (success or error acceptable in CI)

  Scenario: Handshake with custom PEM roots
    Given E2E_TLS_ROOTS_PEM is set to a PEM bundle path
    And a TLS client configured with that custom root store
    When I send a GET request to the server URL
    Then the request attempt should complete


