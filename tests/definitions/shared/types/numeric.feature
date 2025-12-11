@python
Feature: Numeric Types Support

  @python
  Scenario: should cast NUMBER and its synonyms to appropriate type and preserve values when selecting literals
    Given Snowflake client is logged in
    When Query selecting literal values of NUMBER, DECIMAL, DEC, NUMERIC types is executed
    Then All returned values should be of appropriate type
    And All returned values should be equal to the expected literals

  @python
  Scenario: should cast NUMBER and its synonyms to appropriate type and preserve values when selecting from table
    Given Snowflake client is logged in
    And A table with columns of types NUMBER, DECIMAL, DEC, NUMERIC is created
    And Data is inserted into the table
    When Query selecting data from the table is executed
    Then All returned values should be of appropriate type
    And All returned values should be equal to the inserted values

  @python
  Scenario: should handle maximum precision values of NUMBER correctly
    Given Snowflake client is logged in
    When Query "SELECT 1.2345678901234567890123456789012345678::NUMBER(38,37) as max_precision_col" is executed
    And Query "SELECT 99999999999999999999999999999999999999::NUMBER(38,0) as max_value_col" is executed
    And Query "SELECT -99999999999999999999999999999999999999::NUMBER(38,0) as min_value_col" is executed
    Then All queries should return expected values

  @python
  Scenario: should cast INT and its synonyms to appropriate type and preserve values when selecting literals
    Given Snowflake client is logged in
    When Query selecting literal values of INT, INTEGER, BIGINT, SMALLINT, TINYINT, BYTEINT types is executed
    Then All returned values should be cast to integers
    And All returned values should be equal to the expected literals

  @python
  Scenario: should cast INT and its synonyms to appropriate type and preserve values when selecting from table
    Given Snowflake client is logged in
    And A table with columns of types INT, INTEGER, BIGINT, SMALLINT, TINYINT, BYTEINT is created
    And Data is inserted into the table
    When Query selecting data from the table is executed
    Then All returned values should be cast to integers
    And All returned values should be equal to the inserted values

  @python
  Scenario: should handle maximum values of INT correctly
    Given Snowflake client is logged in
    When Query "SELECT 99999999999999999999999999999999999999::INT as max_value_col" is executed
    And Query "SELECT -99999999999999999999999999999999999999::INT as min_value_col" is executed
    Then All queries should return expected integer values

  @python
  Scenario: Type mappings for numeric types are tested
    Given wrapper implements numeric types
    Then type mapping for NUMBER should be tested
    And type mapping for INT should be tested
    And type mapping for FLOAT should be tested
    And type mapping for DECFLOAT should be tested