Feature: Integer type selection and data operations

  Scenario: Fetch data from table
    Given Snowflake client is logged in
    When Query "SELECT 1" is executed
    Then Result should contain [1]

  Scenario: Should return NULL for missing column
    Given Snowflake client is logged in
    When Query "SELECT NULL" is executed
    Then Result should contain [NULL]

  Scenario: should insert and then update and then delete
    Given Snowflake client is logged in
    When Query "INSERT INTO t VALUES (1)" is executed
    Then Result should have 1 row affected
    When Query "UPDATE t SET col = 2 WHERE col = 1" is executed
    Then Result should have 1 row affected
    When Query "DELETE FROM t WHERE col = 2" is executed
    Then Result should have 1 row affected

  Scenario: should select literal for INT
    Given Snowflake client is logged in
    When Query "SELECT 42::INT" is executed
    Then Result should contain [42]

  Scenario: should select literal for BIGINT
    Given Snowflake client is logged in
    When Query "SELECT 42::BIGINT" is executed
    Then Result should contain [42]

  Scenario: should select literal for SMALLINT
    Given Snowflake client is logged in
    When Query "SELECT 42::SMALLINT" is executed
    Then Result should contain [42]

  Scenario: should select literal for TINYINT
    Given Snowflake client is logged in
    When Query "SELECT 42::TINYINT" is executed
    Then Result should contain [42]

  Scenario: should handle various boundary values
    Given Snowflake client is logged in
    When Query "SELECT -128::INT" is executed
    Then Result should contain [-128]
    When Query "SELECT 127::INT" is executed
    Then Result should contain [127]
    When Query "SELECT 0::INT" is executed
    Then Result should contain [0]
    When Query "SELECT 255::INT" is executed
    Then Result should contain [255]

  Scenario: should execute simple query with login
    Given Snowflake client is logged in
    When Query "SELECT 'hello'" is executed
    Then Result should contain ['hello']

  Scenario: should execute another simple query without login
    When Query "SELECT 'world'" is executed
    Then Result should contain ['world']

  Scenario: should upload file to stage
    Given Snowflake client is logged in
    When File "data.csv" is uploaded to stage "@mystage"
    Then Upload should succeed
