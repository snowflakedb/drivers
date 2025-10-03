@core
Feature: TLS connection initialization

  @core_e2e
  Scenario: Should initialize connection with TLS options
    Given TLS certificate and hostname verification are enabled
    And connection parameters (account, user, password, host) are set
    When Connection is initialized
    Then Login should succeed