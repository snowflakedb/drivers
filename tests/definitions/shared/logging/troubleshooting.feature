@python @jdbc @odbc
Feature: Troubleshooting mode

  If the troubleshooting mode is enabled the driver writes all log events
  (regardless of the configured log level) to a dedicated file in the directory
  specified by proper parameter.

  @python_e2e @jdbc_e2e @odbc_e2e
  Scenario: should create troubleshooting log file when enabled via environment variable
    Given SNOWFLAKE_TROUBLESHOOTING_ENABLED is set to "true" and SNOWFLAKE_TROUBLESHOOTING_REPORT_PATH points to a temporary directory
    When a connection is established and a query is executed
    Then a troubleshooting log file exists in the configured directory
    And the log file contains debug-level entries below the configured log level