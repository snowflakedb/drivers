@python
Feature: Session Management

  Session parameter setting and context switching (database, schema).
  Used by every consumer: CLI sets TIMEZONE, Snowfort sets 20+ parameters,
  SQLAlchemy sets QUERY_TAG, Snowpark sets multiple session params.

  # ============================================================================
  # SESSION PARAMETERS
  # ============================================================================

  @python_e2e
  Scenario: should set and verify session parameter QUERY_TAG
    Given Snowflake client is logged in
    When Session parameter QUERY_TAG is set to "e2e_test" via ALTER SESSION
    Then SHOW PARAMETERS LIKE 'QUERY_TAG' should return value "e2e_test"

  @python_e2e
  Scenario: should set and verify session parameter TIMEZONE
    Given Snowflake client is logged in
    When Session parameter TIMEZONE is set to "America/New_York" via ALTER SESSION
    Then SHOW PARAMETERS LIKE 'TIMEZONE' should return value "America/New_York"

  @python_e2e
  Scenario: should alter session parameter at runtime
    Given Snowflake client is logged in
    And Session parameter TIMEZONE is set to "America/New_York"
    When TIMEZONE is changed to "UTC" via ALTER SESSION
    Then SHOW PARAMETERS LIKE 'TIMEZONE' should return value "UTC"

  # ============================================================================
  # CONTEXT SWITCHING
  # ============================================================================

  @python_e2e
  Scenario: should switch role and restore original
    Given Snowflake client is logged in
    And The current role is recorded
    When USE ROLE PUBLIC is executed
    Then SELECT CURRENT_ROLE() should return "PUBLIC"
    When The original role is restored
    Then SELECT CURRENT_ROLE() should return the original role

  @python_e2e
  Scenario: should switch and restore schema context
    Given Snowflake client is logged in
    And The current schema is recorded
    When USE SCHEMA is executed to switch to INFORMATION_SCHEMA
    Then SELECT CURRENT_SCHEMA() should return "INFORMATION_SCHEMA"
    When The original schema is restored via USE SCHEMA
    Then SELECT CURRENT_SCHEMA() should return the original schema
