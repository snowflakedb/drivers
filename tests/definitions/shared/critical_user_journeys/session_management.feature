@python
Feature: Session Management

  Session parameter setting and context switching (database, schema).
  Used by every consumer: CLI sets TIMEZONE, Snowfort sets 20+ parameters,
  SQLAlchemy sets QUERY_TAG, Snowpark sets multiple session params.

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
