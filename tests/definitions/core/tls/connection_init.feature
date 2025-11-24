@core
Feature: TLS connection initialization

  @core_e2e
  Scenario: Should initialize connection with TLS options
    Given Connection parameters are set
    And TLS certificate and hostname verification are enabled
    When Connection is initialized
    Then Login should succeed