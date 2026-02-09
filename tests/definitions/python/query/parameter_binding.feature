@python
Feature: Parameter binding (Python-specific)

  @python_e2e
  Scenario: should bind basic types with positional parameters using ? placeholder
    Given Snowflake client is logged in
    When Query "SELECT ?, ?, ?, ?, ?" is executed with positional parameters [42, 3.14, "hello", True, None]
    Then Result should contain values matching the bound parameters

  @python_e2e
  Scenario: should bind basic types with named parameters using :name placeholder
    Given Snowflake client is logged in
    When Query "SELECT :int_val, :float_val, :str_val, :bool_val, :null_val" is executed with named parameters
    Then Result should contain values matching the bound named parameters

  @python_e2e
  Scenario: should bind positional parameters with numeric placeholders :1, :2, :3
    Given Snowflake client is logged in
    When Query "SELECT :1, :2, :3" is executed with positional parameters [100, "test", True]
    Then Result should contain values in order [100, "test", True]

  @python_e2e
  Scenario: should bind bytes type as binary data
    Given Snowflake client is logged in
    When Query "SELECT ?::BINARY" is executed with bytes parameter b"Hello"
    Then Result should contain binary value b"Hello"

  @python_e2e
  Scenario: should bind datetime values
    Given Snowflake client is logged in
    When Query "SELECT ?::TIMESTAMP_NTZ" is executed with datetime parameter
    Then Result should contain the datetime value

  @python_e2e
  Scenario: should bind Decimal values
    Given Snowflake client is logged in
    When Query "SELECT ?::NUMBER(38,2)" is executed with Decimal parameter
    Then Result should contain the Decimal value

  @python_e2e
  Scenario: should insert single row with parameter binding
    Given Snowflake client is logged in
    And A temporary table with columns (id NUMBER, name VARCHAR, active BOOLEAN) exists
    When Row with values [1, "Alice", True] is inserted using parameter binding
    And Query "SELECT * FROM table" is executed
    Then Result should contain the inserted row

  @python_e2e
  Scenario: should insert multiple rows using executemany
    Given Snowflake client is logged in
    And A temporary table with columns (id NUMBER, name VARCHAR) exists
    When Multiple rows are inserted using executemany with parameters [[1, "Alice"], [2, "Bob"], [3, "Charlie"]]
    Then Query "SELECT * FROM table ORDER BY id" should return 3 rows with correct values

  @python_e2e
  Scenario: should handle NULL values in parameter binding
    Given Snowflake client is logged in
    When Query "SELECT ?, ?, ?" is executed with parameters [None, 42, None]
    Then Result should contain [NULL, 42, NULL]

  @python_e2e
  Scenario: should handle empty string in parameter binding
    Given Snowflake client is logged in
    When Query "SELECT ?::VARCHAR" is executed with parameter ""
    Then Result should contain empty string

  @python_e2e
  Scenario: should handle special characters in string binding
    Given Snowflake client is logged in
    When Query "SELECT ?::VARCHAR" is executed with parameter containing special characters
    Then Result should contain the exact special character string

  @python_e2e
  Scenario: should handle Unicode characters in parameter binding
    Given Snowflake client is logged in
    When Query "SELECT ?::VARCHAR, ?::VARCHAR" is executed with parameters ["日本語", "⛄"]
    Then Result should contain Unicode strings ["日本語", "⛄"]

  @python_e2e
  Scenario: should bind large integer values
    Given Snowflake client is logged in
    When Query "SELECT ?::NUMBER(38,0)" is executed with large integer parameter
    Then Result should contain the large integer value

  @python_e2e
  Scenario: should bind negative numbers
    Given Snowflake client is logged in
    When Query "SELECT ?, ?, ?" is executed with parameters [-42, -3.14, -999999]
    Then Result should contain negative values

  @python_e2e
  Scenario: should bind zero values
    Given Snowflake client is logged in
    When Query "SELECT ?, ?, ?" is executed with parameters [0, 0.0, ""]
    Then Result should contain zero and empty values

  @python_e2e
  Scenario: should handle mixed positional and type casting
    Given Snowflake client is logged in
    When Query "SELECT ?::NUMBER, ?::VARCHAR, ?::BOOLEAN" is executed with parameters [42, "hello", True]
    Then Result should match the type-casted parameters

  @python_e2e
  Scenario: should be backward compatible with old connector parameter format
    Given Snowflake client is logged in
    When Query is executed with parameters in old connector format
    Then Result should be identical to new format
