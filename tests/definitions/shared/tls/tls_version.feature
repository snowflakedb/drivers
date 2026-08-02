@core @odbc @python
Feature: TLS version enforcement

  @core_e2e @odbc_e2e @python_e2e
  Scenario: should negotiate TLS when the server offers a version inside the window
    Given a TLS server that offers only TLS 1.3
    And a client configured with min_tls_version tls12 and max_tls_version tls13
    When a request is sent to the server
    Then the handshake succeeds

  @core_e2e @odbc_e2e @python_e2e
  Scenario: should fail the handshake when the server only offers a version below the minimum
    Given a TLS server that offers only TLS 1.2
    And a client configured with min_tls_version tls13
    When a request is sent to the server
    Then the handshake fails

  @core_e2e @odbc_e2e @python_e2e
  Scenario: should reject the configuration when the minimum exceeds the maximum
    Given settings with min_tls_version tls13 and max_tls_version tls12
    When the TLS configuration is built from settings
    Then a configuration error is returned
