@core
Feature: JSON Result Set

  @core_e2e
  Scenario: should return arrow even if JSON result set is returned for simple types
    Given Snowflake client is logged in
    When Table json_result_set_simple_types (str_col STRING, tinyint_col TINYINT, smallint_col SMALLINT, int_col INT, bigint_col BIGINT, number_scale_0_col NUMBER(38, 0), number_scale_2_col NUMBER(38, 2)) is created
    And Row is inserted with INSERT INTO json_result_set_simple_types VALUES ('abc', 123, 12345, 1234567, 12345678901234567890, 12345678901234567890123456789012345678, 123.45)
    And Query "SELECT * FROM json_result_set_simple_types" is executed
    And Query result format is forced to JSON
    And Query "SELECT * FROM json_result_set_simple_types" is executed
    Then Schema for both queries should match
    And the result for both queries should match
    And Statement should be released

  # TODO add a test for larger result set with chunks

  # TODO add a test for all possible data types
  # can we modify this test later to return other data types as well? or do we need a separate test for that?
