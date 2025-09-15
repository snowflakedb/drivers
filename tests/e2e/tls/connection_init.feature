Feature: TLS connection initialization
  As a client of the universal driver
  I want to initialize a connection with TLS verification enabled
  So that I can securely connect to Snowflake

  @core
  Scenario: connection init with TLS options succeeds
    Given TLS certificate and hostname verification are enabled
    And connection parameters (account, user, password, host) are set
    When I initialize the connection
    Then the login succeeds
