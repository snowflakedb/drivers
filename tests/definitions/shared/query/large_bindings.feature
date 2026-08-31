@odbc @python @jdbc
Feature: Large (stage-based) parameter binding

  @odbc_e2e @python_e2e @jdbc_e2e
  Scenario: should stage-bind at the default threshold and reuse SYSTEM$BIND across consecutive bulk inserts
    Given Snowflake client is logged in
    And A temporary table with columns (id NUMBER, name VARCHAR) exists
    When 33000 rows generated as [[i, "first-" + i] for i in 0..33000] are inserted using multirow binding
    Then the bind file on SYSTEM$BIND from the last bulk insert should contain the same values as the bound parameters
    When 33000 rows generated as [[33000 + i, "second-" + i] for i in 0..33000] are inserted using multirow binding
    Then the bind file on SYSTEM$BIND from the last bulk insert should contain the same values as the bound parameters
    And Query "SELECT id, name FROM {table} ORDER BY id" is executed
    Then Result should contain the same values as the bound parameters from both bulk inserts

  @odbc_e2e @python_e2e @jdbc_e2e
  Scenario: should round-trip all bindable types via stage binding
    Given Snowflake client is logged in
    And A temporary table with columns (id NUMBER, n NUMBER, f FLOAT, flag BOOLEAN, txt VARCHAR) exists
    When 13200 rows are inserted using multirow binding
    Then the bind file on SYSTEM$BIND from the last bulk insert should contain the same values as the bound parameters
    And Query "SELECT id, n, f, flag, txt FROM {table} ORDER BY id" is executed
    Then Result should contain the same values as the bound parameters

  @odbc_e2e @python_e2e @jdbc_e2e
  Scenario: should preserve CSV escaping hazards via stage binding
    Given Snowflake client is logged in
    And A temporary table with columns (id NUMBER, txt VARCHAR) exists
    When 33000 rows are inserted using multirow binding with values cycling every 7 rows through [[0, "val,0"], [1, "say\"1\""], [2, "a\nb"], [3, "C:\\dir\\3"], [4, ""], [5, NULL], [6, "日本語"]]
    Then the bind file on SYSTEM$BIND from the last bulk insert should contain the same values as the bound parameters
    And Query "SELECT id, txt FROM {table} WHERE id BETWEEN 0 AND 6 ORDER BY id" is executed
    Then Result should contain rows [[0, "val,0"], [1, "say\"1\""], [2, "a\nb"], [3, "C:\\dir\\3"], [4, ""], [5, NULL], [6, "日本語"]]

  @odbc_e2e @python_e2e @jdbc_e2e
  Scenario: should not stage-bind scalar or non-INSERT queries even when threshold is crossed
    Given Snowflake client is logged in
    And CLIENT_STAGE_ARRAY_BINDING_THRESHOLD session parameter is set to 1
    When "SELECT ? AS val" is executed with bound integer value 42
    Then the bind file on SYSTEM$BIND from the last execute should not contain the bound parameter values
    And the result should equal 42

  @odbc_e2e @python_e2e @jdbc_e2e
  Scenario: should use inline JSON when row count is below CLIENT_STAGE_ARRAY_BINDING_THRESHOLD
    Given Snowflake client is logged in
    And A temporary table with columns (id NUMBER, name VARCHAR) exists
    And CLIENT_STAGE_ARRAY_BINDING_THRESHOLD session parameter is set to 100
    When 10 rows generated as [[i, "json-" + i] for i in 0..10] are inserted using multirow binding
    Then no new bind file should have been uploaded to SYSTEM$BIND
    And Query "SELECT id, name FROM {table} WHERE id IN (0, 9) ORDER BY id" is executed
    Then Result should contain rows [[0, "json-0"], [9, "json-9"]]

  @odbc_e2e @python_e2e @jdbc_e2e
  Scenario: should use stage binding at exact threshold boundary
    Given Snowflake client is logged in
    And A temporary table with columns (id NUMBER, name VARCHAR) exists
    And CLIENT_STAGE_ARRAY_BINDING_THRESHOLD session parameter is set to 20
    When 10 rows generated as [[i, "stage-" + i] for i in 0..10] are inserted using multirow binding
    Then the bind file on SYSTEM$BIND from the last bulk insert should contain the same values as the bound parameters
    And Query "SELECT id, name FROM {table} WHERE id IN (0, 9) ORDER BY id" is executed
    Then Result should contain rows [[0, "stage-0"], [9, "stage-9"]]

  @odbc_e2e @jdbc_e2e
  Scenario: should keep an all-NULL row on the inline JSON path when stage binding is disabled
    Given Snowflake client is logged in
    And A temporary table with columns (id INTEGER, colA DOUBLE, colB FLOAT, colC VARCHAR, colD NUMBER, colE INTEGER) exists
    And CLIENT_STAGE_ARRAY_BINDING_THRESHOLD session parameter is set to 0
    When a batch of one row with every column set to SQL NULL is inserted using multirow binding
    Then no new bind file should have been uploaded to SYSTEM$BIND
    And every column of the round-tripped row reads back as SQL NULL

  @odbc_e2e @jdbc_e2e
  Scenario: should stage-bind an all-NULL row when the bound cell count meets the threshold
    Given Snowflake client is logged in
    And A temporary table with columns (id INTEGER, colA DOUBLE, colB FLOAT, colC VARCHAR, colD NUMBER, colE INTEGER) exists
    And CLIENT_STAGE_ARRAY_BINDING_THRESHOLD session parameter is set to 6
    When a batch of one row with every column set to SQL NULL is inserted using multirow binding
    Then the bind file on SYSTEM$BIND from the last bulk insert should contain the same values as the bound parameters
    And every column of the round-tripped row reads back as SQL NULL

  @odbc_e2e
  Scenario: should skip SQL_PARAM_IGNORE sets during array execution
    Given Snowflake client is logged in
    And A temporary table with an id column exists
    When 5 sets {10, 20, 30, 40, 50} are inserted with the 2nd and 4th marked SQL_PARAM_IGNORE
    Then SQL_ATTR_PARAMS_PROCESSED_PTR reports all 5 sets and the status array marks ignored sets SQL_PARAM_UNUSED
    And Query "SELECT id FROM {table} ORDER BY id" is executed
    Then Result should contain only the proceeded rows [10, 30, 50]

  @odbc_e2e
  Scenario: should skip SQL_PARAM_IGNORE sets with an explicit APP_PARAM_DESC
    Given Snowflake client is logged in
    And A temporary table with an id column exists
    And An explicit SQL_ATTR_APP_PARAM_DESC is assigned to the statement
    When 5 sets {10, 20, 30, 40, 50} are inserted with the 2nd and 4th marked SQL_PARAM_IGNORE
    Then SQL_ATTR_PARAMS_PROCESSED_PTR reports all 5 sets and the status array marks ignored sets SQL_PARAM_UNUSED
    And Query "SELECT id FROM {table} ORDER BY id" is executed
    Then Result should contain only the proceeded rows [10, 30, 50]

  # TODO(SNOW-3235553): add a scenario for the all-ignored edge case (every set
  # marked SQL_PARAM_IGNORE -> zero rows -> empty INSERT). Deferred until the
  # server's response to an empty payload (error vs no-op) is verified; the
  # driver-side path is already covered by the
  # `json_all_param_ignore_yields_empty_value_arrays` unit test.

  @python_e2e
  Scenario: should fall back to per-row execution for non-INSERT statements
    Given Snowflake client is logged in
    And A temporary table with columns (id NUMBER, name VARCHAR) exists
    And CLIENT_STAGE_ARRAY_BINDING_THRESHOLD session parameter is set to 1
    When an UPDATE with array bindings above the threshold is executed via executemany
    Then all updated rows reflect the new values
    And no new bind file should have been uploaded

  @python_e2e
  Scenario: should round-trip far-future dates via stage binding
    Given Snowflake client is logged in
    And A temporary table with columns (id NUMBER, d DATE) exists
    And CLIENT_STAGE_ARRAY_BINDING_THRESHOLD session parameter is set to 1
    When dates spanning the epoch-millisecond overflow bound are inserted using multirow binding
    And Query "SELECT id, d FROM {table} ORDER BY id" is executed
    Then Result should contain the same dates as the bound parameters

