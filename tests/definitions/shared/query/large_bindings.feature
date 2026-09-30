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
    # FIXED/NUMBER (integer and BigDecimal), REAL/FLOAT, BOOLEAN, TEXT/VARCHAR,
    # BINARY, DATE, TIME, TIMESTAMP_LTZ, and TIMESTAMP_NTZ.
    Given Snowflake client is logged in
    And A temporary table with the driver-specific stage-binding type matrix exists
    When 13200 rows of driver-specific stage-binding values are inserted using multirow binding
    Then the bind file on SYSTEM$BIND from the last bulk insert should contain the same values as the bound parameters
    And All type-matrix columns are selected from the table in row order
    Then Result should contain the same values as the bound parameters

  @jdbc_e2e
  Scenario: should reject a stage-bound TIMESTAMP_TZ offset pair on the old and new JDBC drivers
    # CLIENT_TIMESTAMP_TYPE_MAPPING = TIMESTAMP_TZ makes setTimestamp(index, value, Calendar)
    # emit the `<epoch_nanos> <offset_minutes + 1440>` pair that only the inline JSON bind
    # protocol decodes. The staged file is untyped CSV, so the server reads the cell as a
    # timestamp literal and rejects it.
    Given Snowflake client is logged in
    And A temporary table with columns (id NUMBER, ts_tz TIMESTAMP_TZ) exists
    And CLIENT_TIMESTAMP_TYPE_MAPPING session parameter is set to TIMESTAMP_TZ
    And CLIENT_STAGE_ARRAY_BINDING_THRESHOLD session parameter is set to 4
    When 10 rows carrying a non-UTC offset are inserted using multirow binding with an explicit Calendar
    Then the bulk insert should fail with SQLSTATE 22007

  @jdbc_e2e
  Scenario: should preserve the TIMESTAMP_TZ instant with path-specific offsets
    # Under the default CLIENT_TIMESTAMP_TYPE_MAPPING a plain setTimestamp binds as TIMESTAMP_LTZ,
    # which the server coerces into a TIMESTAMP_TZ column. Inline, the bind travels as epoch nanos
    # and the server attaches the session offset; staged, it is formatted into a literal carrying
    # the JVM offset.
    Given Snowflake client is logged in
    And A temporary table with columns (id NUMBER, ts_tz TIMESTAMP_TZ) exists
    And the session timezone differs from the JVM timezone
    When 10 rows are inserted using multirow binding below CLIENT_STAGE_ARRAY_BINDING_THRESHOLD
    And 10 rows carrying the same instant are inserted using multirow binding above CLIENT_STAGE_ARRAY_BINDING_THRESHOLD
    And Query "SELECT id, ts_tz, TO_VARCHAR(ts_tz, 'TZHTZM') FROM {table} ORDER BY id" is executed
    Then every row should hold the bound instant
    And the inline rows should use the session offset while the staged rows use the JVM offset

  @odbc_e2e @python_e2e @jdbc_e2e
  Scenario: should preserve CSV escaping hazards via stage binding
    Given Snowflake client is logged in
    And A temporary table with columns (id NUMBER, txt VARCHAR) exists
    When 33000 rows are inserted using multirow binding with values cycling every 16 rows through [[0, "val,0"], [1, "say\"1\""], [2, "a\nb"], [3, "C:\\dir\\3"], [4, ""], [5, NULL], [6, "日本語"], [7, "\""], [8, ","], [9, "\n"], [10, "\r\n"], [11, "\"\""], [12, "null"], [13, "\\\n"], [14, "\","], [15, "\\\",\\\""]]
    Then the bind file on SYSTEM$BIND from the last bulk insert should contain the same values as the bound parameters
    And Query "SELECT id, txt FROM {table} WHERE id BETWEEN 0 AND 15 ORDER BY id" is executed
    Then Result should contain rows [[0, "val,0"], [1, "say\"1\""], [2, "a\nb"], [3, "C:\\dir\\3"], [4, ""], [5, NULL], [6, "日本語"], [7, "\""], [8, ","], [9, "\n"], [10, "\r\n"], [11, "\"\""], [12, "null"], [13, "\\\n"], [14, "\","], [15, "\\\",\\\""]]

  @jdbc_e2e
  Scenario Outline: should reject invalid numeric text at the configured batch threshold
    Given Snowflake client is logged in
    And A temporary table with columns (id INTEGER, value INTEGER) exists
    When "notAnInt" is batch-bound into the numeric column at threshold <threshold>
    Then the batch execution should fail with SQLException

    Examples:
      | threshold |
      | 0         |
      | 2         |

  @jdbc_e2e
  Scenario Outline: should resolve NULL and FLOAT array-bind types on inline and stage paths
    Given Snowflake client is logged in
    And A temporary table with columns (id INTEGER, value FLOAT) exists
    When NULL values declared as NUMERIC, BOOLEAN, and CHAR are batched with FLOAT values at threshold <threshold>
    Then NULL and FLOAT values should round-trip
    And SYSTEM$BIND usage should match threshold <threshold>
    And adding a STRING value to the FLOAT batch should fail with SQLSTATE 0A000 and vendor code 200023

    Examples:
      | threshold |
      | 0         |
      | 1         |

  @jdbc_e2e
  Scenario: should stage-bind timestamp strings into TZ and NTZ columns
    Given Snowflake client is logged in
    And A temporary table with columns (ts_tz TIMESTAMP_TZ, ts_ntz TIMESTAMP_NTZ) exists
    And CLIENT_STAGE_ARRAY_BINDING_THRESHOLD session parameter is set to 1
    When timestamp strings with explicit and implicit offsets are inserted using multirow binding
    Then the bind file on SYSTEM$BIND from the last bulk insert should contain the same values as the bound parameters
    And the TZ and NTZ values should preserve their expected timestamps and offsets

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

  @odbc_e2e @python_e2e @jdbc_e2e
  Scenario: should keep an all-NULL row on the inline JSON path when stage binding is disabled
    Given Snowflake client is logged in
    And A temporary table with columns (id INTEGER, colA DOUBLE, colB FLOAT, colC VARCHAR, colD NUMBER, colE INTEGER) exists
    And CLIENT_STAGE_ARRAY_BINDING_THRESHOLD session parameter is set to 0
    When a batch of one row with every column set to SQL NULL is inserted using multirow binding
    Then no new bind file should have been uploaded to SYSTEM$BIND
    And every column of the round-tripped row reads back as SQL NULL

  @odbc_e2e @python_e2e @jdbc_e2e
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

  @python_e2e @jdbc_e2e
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

