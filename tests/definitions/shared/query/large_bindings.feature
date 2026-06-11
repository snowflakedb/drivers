@odbc
Feature: Large (stage-based) parameter binding

  @odbc_e2e
  Scenario: should stage-bind at the default threshold and reuse SYSTEM$BIND across consecutive bulk inserts
    Given Snowflake client is logged in
    And A temporary table with columns (id NUMBER, name VARCHAR) exists
    When 33000 rows generated as [[i, "first-" + i] for i in 0..33000] are inserted using multirow binding
    Then the bind file on SYSTEM$BIND from the last bulk insert should contain the same values as the bound parameters
    When 33000 rows generated as [[33000 + i, "second-" + i] for i in 0..33000] are inserted using multirow binding
    Then the bind file on SYSTEM$BIND from the last bulk insert should contain the same values as the bound parameters
    And Query "SELECT id, name FROM {table} ORDER BY id" is executed
    Then Result should contain the same values as the bound parameters from both bulk inserts

  @odbc_e2e
  Scenario: should round-trip all bindable types via stage binding
    Given Snowflake client is logged in
    And A temporary table with columns (id NUMBER, n NUMBER, f FLOAT, flag BOOLEAN, txt VARCHAR) exists
    When 13200 rows are inserted using multirow binding
    Then the bind file on SYSTEM$BIND from the last bulk insert should contain the same values as the bound parameters
    And Query "SELECT id, n, f, flag, txt FROM {table} ORDER BY id" is executed
    Then Result should contain the same values as the bound parameters

  @odbc_e2e
  Scenario: should preserve CSV escaping hazards via stage binding
    Given Snowflake client is logged in
    And A temporary table with columns (id NUMBER, txt VARCHAR) exists
    When 33000 rows are inserted using multirow binding with values cycling every 7 rows through [[0, "val,0"], [1, "say\"1\""], [2, "a\nb"], [3, "C:\\dir\\3"], [4, ""], [5, NULL], [6, "日本語"]]
    Then the bind file on SYSTEM$BIND from the last bulk insert should contain the same values as the bound parameters
    And Query "SELECT id, txt FROM {table} WHERE id BETWEEN 0 AND 6 ORDER BY id" is executed
    Then Result should contain rows [[0, "val,0"], [1, "say\"1\""], [2, "a\nb"], [3, "C:\\dir\\3"], [4, ""], [5, NULL], [6, "日本語"]]

  @odbc_e2e
  Scenario: should fall back to JSON when arrayBindSupported is false despite crossing the threshold
    Given Snowflake client is logged in
    And CLIENT_STAGE_ARRAY_BINDING_THRESHOLD session parameter is set to 1
    When "SELECT ? AS val" is executed with bound integer value 42
    Then the bind file on SYSTEM$BIND from the last execute should not contain the bound parameter values
    And the result should equal 42

  @odbc_e2e
  Scenario: should use inline JSON when row count is below CLIENT_STAGE_ARRAY_BINDING_THRESHOLD
    Given Snowflake client is logged in
    And A temporary table with columns (id NUMBER, name VARCHAR) exists
    And CLIENT_STAGE_ARRAY_BINDING_THRESHOLD session parameter is set to 100
    When 10 rows generated as [[i, "json-" + i] for i in 0..10] are inserted using multirow binding
    Then no new bind file should have been uploaded to SYSTEM$BIND
    And Query "SELECT id, name FROM {table} WHERE id IN (0, 9) ORDER BY id" is executed
    Then Result should contain rows [[0, "json-0"], [9, "json-9"]]

  @odbc_e2e
  Scenario: should use stage binding at exact threshold boundary
    Given Snowflake client is logged in
    And A temporary table with columns (id NUMBER, name VARCHAR) exists
    And CLIENT_STAGE_ARRAY_BINDING_THRESHOLD session parameter is set to 20
    When 10 rows generated as [[i, "stage-" + i] for i in 0..10] are inserted using multirow binding
    Then the bind file on SYSTEM$BIND from the last bulk insert should contain the same values as the bound parameters
    And Query "SELECT id, name FROM {table} WHERE id IN (0, 9) ORDER BY id" is executed
    Then Result should contain rows [[0, "stage-0"], [9, "stage-9"]]
