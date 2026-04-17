@core
Feature: Client platform detection in login request
  As a driver user
  I want the driver to report runtime platform identifiers to Snowflake
  So that telemetry can distinguish where clients are running

  @core_int
  Scenario: should send PLATFORM disabled when detection is disabled via env var
    Given SNOWFLAKE_DISABLE_PLATFORM_DETECTION is set to "true"
    And Wiremock is running with a password login-success mapping
    When Trying to Connect
    Then The login-request body contains CLIENT_ENVIRONMENT.PLATFORM equal to ["disabled"]

  @core_int
  Scenario: should send empty PLATFORM array when detection produces no platforms
    Given SNOWFLAKE_DISABLE_PLATFORM_DETECTION is unset
    And Wiremock is running with a password login-success mapping
    When Trying to Connect
    Then The login-request body contains CLIENT_ENVIRONMENT.PLATFORM equal to []
