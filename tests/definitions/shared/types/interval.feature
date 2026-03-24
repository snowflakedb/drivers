@odbc
Feature: INTERVAL datatype handling
  # Snowflake INTERVAL types: INTERVAL YEAR, INTERVAL MONTH, INTERVAL YEAR TO MONTH,
  # INTERVAL DAY, INTERVAL HOUR, INTERVAL MINUTE, INTERVAL SECOND,
  # INTERVAL DAY TO HOUR, INTERVAL DAY TO MINUTE, INTERVAL DAY TO SECOND,
  # INTERVAL HOUR TO MINUTE, INTERVAL HOUR TO SECOND, INTERVAL MINUTE TO SECOND.
  #
  # YEAR/MONTH family is represented as a signed number of months.
  # DAY/TIME family is represented as a signed duration with day, hour, minute,
  # second and fractional second components.
  #
  # Note: the docs state INTERVAL is not a data type, but Snowflake does support
  # it as a full column type (public preview, gated by ENABLE_INTERVAL_TYPE).
  # Reference: https://docs.snowflake.com/en/sql-reference/data-types-datetime#interval
  #
  # Both JSON and Arrow fully support INTERVAL as a distinct type
  # (INTERVAL_YEAR_MONTH / INTERVAL_DAY_TIME). Arrow physical vectors are int64
  # (months or nanos) or Decimal128 (large nanos). The GS parameter
  # ENABLE_INTERVAL_TYPES_AS_TEXT_IN_CLIENT_RESPONSE (session-settable,
  # default false) switches to formatted text strings with TEXT metadata for
  # drivers without native support.
  #
  # Python >= 3.18.0, JDBC >= 3.27.0 handle the native interval types. The
  # reference ODBC driver does not have native support, so intervals are
  # surfaced as SQL_VARCHAR with numeric string values (total months for
  # YEAR/MONTH family, scaled nanoseconds for DAY/TIME family). Column
  # metadata is identical between default and text-fallback modes.
  #
  # TODO: The UD ODBC driver should map INTERVAL columns to SQL_INTERVAL_*
  # types once Arrow-level interval support is implemented in the Rust driver.
  #
  # Test coverage: These tests cover the default path (native interval
  # metadata with numeric values). The two other backend modes are
  # intentionally not tested:
  #   - Feature off (ENABLE_INTERVAL_TYPE disabled): The SQL engine rejects
  #     interval syntax entirely; nothing to test from the driver side.
  #   - Text fallback (ENABLE_INTERVAL_TYPES_AS_TEXT_IN_CLIENT_RESPONSE = true):
  #     Intervals arrive as formatted VARCHAR strings (e.g., "+1-02",
  #     "+0 00:00:01.000000000"). No interval-specific driver logic is
  #     exercised; it is just standard string passthrough.

  # ============================================================================
  # TYPE CASTING
  # ============================================================================

  @odbc_e2e
  Scenario: should cast INTERVAL values to appropriate type for YEAR TO MONTH and DAY TO SECOND
    # Python: YEAR TO MONTH as canonical string, DAY TO SECOND as timedelta
    # JDBC: YEAR TO MONTH as java.time.Period, DAY TO SECOND as java.time.Duration
    # ODBC: FIXED/NUMBER string via SQL_C_CHAR
    Given Snowflake client is logged in
    When Query "SELECT '1-2'::INTERVAL YEAR TO MONTH, '999999999-11'::INTERVAL YEAR TO MONTH, '0 0:0:1.2'::INTERVAL DAY TO SECOND, '99999 23:59:59.999999'::INTERVAL DAY TO SECOND" is executed
    Then all INTERVAL values should be returned as appropriate type for the driver

  # ============================================================================
  # SELECT LITERALS
  # ============================================================================

  @odbc_e2e
  Scenario: should select INTERVAL YEAR TO MONTH literals
    # Happy path + corner cases for YEAR TO MONTH:
    #   - 0-0
    #   - small positive / negative: 1-2, -1-3
    #   - extremes: 999999999-11, -999999999-11
    Given Snowflake client is logged in
    When Query selecting INTERVAL YEAR TO MONTH literals is executed
    Then the result should contain expected INTERVAL YEAR TO MONTH literal values in order

  @odbc_e2e
  Scenario: should select INTERVAL DAY TO SECOND literals
    # Happy path + corner cases for DAY TO SECOND:
    #   - Zero:          '0 0:0:0.0'
    #   - Positive:      '12 3:4:5.678'
    #   - Negative:      '-1 2:3:4.567'
    #   - Large + frac:  '99999 23:59:59.999999'
    #   - Large negative:'-99999 23:59:59.999999'
    Given Snowflake client is logged in
    When Query selecting INTERVAL DAY TO SECOND literals is executed
    Then the result should contain expected INTERVAL DAY TO SECOND literal values in order

  @odbc_e2e
  Scenario: should select INTERVAL YEAR literals
    # Sub-type of YEAR-MONTH family
    Given Snowflake client is logged in
    When Query "SELECT '0'::INTERVAL YEAR, '1'::INTERVAL YEAR, '-1'::INTERVAL YEAR, '999999999'::INTERVAL YEAR, '-999999999'::INTERVAL YEAR" is executed
    Then the result should contain expected INTERVAL YEAR literal values in order

  @odbc_e2e
  Scenario: should select INTERVAL MONTH literals
    # Sub-type of YEAR-MONTH family
    Given Snowflake client is logged in
    When Query "SELECT '0'::INTERVAL MONTH, '1'::INTERVAL MONTH, '-1'::INTERVAL MONTH, '999999999'::INTERVAL MONTH, '-999999999'::INTERVAL MONTH" is executed
    Then the result should contain expected INTERVAL MONTH literal values in order

  @odbc_e2e
  Scenario: should select INTERVAL DAY literals
    # Sub-type of DAY-TIME family
    Given Snowflake client is logged in
    When Query "SELECT '0'::INTERVAL DAY, '1'::INTERVAL DAY, '-1'::INTERVAL DAY, '999999999'::INTERVAL DAY, '-999999999'::INTERVAL DAY" is executed
    Then the result should contain expected INTERVAL DAY literal values in order

  @odbc_e2e
  Scenario: should select INTERVAL HOUR literals
    Given Snowflake client is logged in
    When Query "SELECT '0'::INTERVAL HOUR, '1'::INTERVAL HOUR, '-1'::INTERVAL HOUR, '999999999'::INTERVAL HOUR, '-999999999'::INTERVAL HOUR" is executed
    Then the result should contain expected INTERVAL HOUR literal values in order

  @odbc_e2e
  Scenario: should select INTERVAL MINUTE literals
    Given Snowflake client is logged in
    When Query "SELECT '0'::INTERVAL MINUTE, '1'::INTERVAL MINUTE, '-1'::INTERVAL MINUTE, '999999999'::INTERVAL MINUTE, '-999999999'::INTERVAL MINUTE" is executed
    Then the result should contain expected INTERVAL MINUTE literal values in order

  @odbc_e2e
  Scenario: should select INTERVAL SECOND literals
    Given Snowflake client is logged in
    When Query "SELECT '0'::INTERVAL SECOND, '1.0'::INTERVAL SECOND, '-1.0'::INTERVAL SECOND, '999999999.999999'::INTERVAL SECOND, '-999999999.999999'::INTERVAL SECOND" is executed
    Then the result should contain expected INTERVAL SECOND literal values in order

  @odbc_e2e
  Scenario: should select INTERVAL DAY TO HOUR literals
    Given Snowflake client is logged in
    When Query "SELECT '0 0'::INTERVAL DAY TO HOUR, '1 2'::INTERVAL DAY TO HOUR, '-1 2'::INTERVAL DAY TO HOUR, '999999999 23'::INTERVAL DAY TO HOUR, '-999999999 23'::INTERVAL DAY TO HOUR" is executed
    Then the result should contain expected INTERVAL DAY TO HOUR literal values in order

  @odbc_e2e
  Scenario: should select INTERVAL DAY TO MINUTE literals
    Given Snowflake client is logged in
    When Query "SELECT '0 0:0'::INTERVAL DAY TO MINUTE, '1 2:30'::INTERVAL DAY TO MINUTE, '-1 2:30'::INTERVAL DAY TO MINUTE, '999999999 23:59'::INTERVAL DAY TO MINUTE, '-999999999 23:59'::INTERVAL DAY TO MINUTE" is executed
    Then the result should contain expected INTERVAL DAY TO MINUTE literal values in order

  @odbc_e2e
  Scenario: should select INTERVAL HOUR TO MINUTE literals
    Given Snowflake client is logged in
    When Query "SELECT '0:0'::INTERVAL HOUR TO MINUTE, '1:30'::INTERVAL HOUR TO MINUTE, '-1:30'::INTERVAL HOUR TO MINUTE, '999999999:59'::INTERVAL HOUR TO MINUTE, '-999999999:59'::INTERVAL HOUR TO MINUTE" is executed
    Then the result should contain expected INTERVAL HOUR TO MINUTE literal values in order

  @odbc_e2e
  Scenario: should select INTERVAL HOUR TO SECOND literals
    Given Snowflake client is logged in
    When Query "SELECT '0:0:0.0'::INTERVAL HOUR TO SECOND, '1:30:45.123'::INTERVAL HOUR TO SECOND, '-1:30:45.123'::INTERVAL HOUR TO SECOND, '999999999:59:59.999999'::INTERVAL HOUR TO SECOND, '-999999999:59:59.999999'::INTERVAL HOUR TO SECOND" is executed
    Then the result should contain expected INTERVAL HOUR TO SECOND literal values in order

  @odbc_e2e
  Scenario: should select INTERVAL MINUTE TO SECOND literals
    Given Snowflake client is logged in
    When Query "SELECT '0:0.0'::INTERVAL MINUTE TO SECOND, '30:45.123'::INTERVAL MINUTE TO SECOND, '-30:45.123'::INTERVAL MINUTE TO SECOND, '999999999:59.999999'::INTERVAL MINUTE TO SECOND, '-999999999:59.999999'::INTERVAL MINUTE TO SECOND" is executed
    Then the result should contain expected INTERVAL MINUTE TO SECOND literal values in order

  @odbc_e2e
  Scenario: should select NULL INTERVAL literals
    Given Snowflake client is logged in
    When Query "SELECT NULL::INTERVAL YEAR TO MONTH, NULL::INTERVAL DAY TO SECOND, NULL::INTERVAL YEAR, NULL::INTERVAL SECOND" is executed
    Then the result should contain:
      | col1 | col2 | col3 | col4 |
      | NULL | NULL | NULL | NULL |

  @odbc_e2e
  Scenario: should treat INTERVAL without explicit part as seconds
    # INTERVAL '2' is the same as INTERVAL '2 seconds'
    Given Snowflake client is logged in
    When Query "SELECT '2024-04-15 12:00:00'::TIMESTAMP + INTERVAL '2' AS d1, '2024-04-15 12:00:00'::TIMESTAMP + INTERVAL '2 seconds' AS d2" is executed
    Then the result should contain:
      | d1                      | d2                      |
      | 2024-04-15 12:00:02.000 | 2024-04-15 12:00:02.000 |

  # ============================================================================
  # SELECT FROM TABLE
  # ============================================================================
  # The default-precision scenarios below also exercise the largest Arrow storage
  # widths: SB8 (BigIntVector) for YEAR TO MONTH, SB16 (Decimal128) for DAY TO
  # SECOND. The precision-variant scenarios test smaller Arrow vector types.

  @odbc_e2e
  Scenario: should select INTERVAL YEAR TO MONTH values from table
    Given Snowflake client is logged in
    And A temporary table with INTERVAL YEAR TO MONTH column is created
    And The table is populated with YEAR TO MONTH values including corner cases
    # Values include:
    #   - -999999999-11
    #   - -1-3
    #   - 0-0
    #   - 1-2
    #   - 999999999-11
    #   - NULL
    When Query "SELECT * FROM {table} ORDER BY C1 NULLS LAST" is executed
    Then the result should contain the inserted INTERVAL YEAR TO MONTH values in order

  @odbc_e2e
  Scenario: should select INTERVAL DAY TO SECOND values from table
    Given Snowflake client is logged in
    And A temporary table with INTERVAL DAY TO SECOND column is created
    And The table is populated with DAY TO SECOND values including corner cases
    # Values include (same set as literal scenario plus NULL):
    #   - '0 0:0:0.0'
    #   - '12 3:4:5.678'
    #   - '-1 2:3:4.567'
    #   - '99999 23:59:59.999999'
    #   - '-99999 23:59:59.999999'
    #   - NULL
    When Query "SELECT * FROM {table} ORDER BY C1 NULLS LAST" is executed
    Then the result should contain the inserted INTERVAL DAY TO SECOND values in order

  @odbc_e2e
  Scenario: should select INTERVAL YEAR(2) TO MONTH values from table
    # Arrow precision variant: SmallIntVector (SB2, 16-bit)
    Given Snowflake client is logged in
    And A temporary table with INTERVAL YEAR(2) TO MONTH column is created
    And The table is populated with values ['0-0', '1-2', '-1-3', '99-11', '-99-11', NULL]
    When Query "SELECT * FROM {table} ORDER BY C1 NULLS LAST" is executed
    Then the result should contain the inserted INTERVAL YEAR(2) TO MONTH values in order

  @odbc_e2e
  Scenario: should select INTERVAL YEAR(7) TO MONTH values from table
    # Arrow precision variant: IntVector (SB4, 32-bit)
    Given Snowflake client is logged in
    And A temporary table with INTERVAL YEAR(7) TO MONTH column is created
    And The table is populated with values ['0-0', '1-2', '-1-3', '9999999-11', '-9999999-11', NULL]
    When Query "SELECT * FROM {table} ORDER BY C1 NULLS LAST" is executed
    Then the result should contain the inserted INTERVAL YEAR(7) TO MONTH values in order

  @odbc_e2e
  Scenario: should select INTERVAL DAY(3) TO SECOND values from table
    # Arrow precision variant: BigIntVector (SB8, 64-bit nanoseconds)
    Given Snowflake client is logged in
    And A temporary table with INTERVAL DAY(3) TO SECOND column is created
    And The table is populated with values ['0 0:0:0.0', '1 2:3:4.567', '-1 2:3:4.567', '999 23:59:59.999999', '-999 23:59:59.999999', NULL]
    When Query "SELECT * FROM {table} ORDER BY C1 NULLS LAST" is executed
    Then the result should contain the inserted INTERVAL DAY(3) TO SECOND values in order

  # ============================================================================
  # BINDING
  # ============================================================================

  @odbc_e2e
  Scenario: should insert and select back INTERVAL YEAR TO MONTH values using parameter binding
    Given Snowflake client is logged in
    And A temporary table with INTERVAL YEAR TO MONTH column is created
    When INTERVAL YEAR TO MONTH values ['0-0', '1-2', '-1-3', '999999999-11', '-999999999-11', NULL] are inserted using parameter binding
    And Query "SELECT * FROM {table} ORDER BY C1 NULLS LAST" is executed
    Then the result should contain the bound INTERVAL YEAR TO MONTH values ['-999999999-11', '-1-3', '0-0', '1-2', '999999999-11', NULL]

  @odbc_e2e
  Scenario: should insert and select back INTERVAL DAY TO SECOND values using parameter binding
    Given Snowflake client is logged in
    And A temporary table with INTERVAL DAY TO SECOND column is created
    When INTERVAL DAY TO SECOND values ['0 0:0:0.0', '12 3:4:5.678', '-1 2:3:4.567', '99999 23:59:59.999999', '-99999 23:59:59.999999', NULL] are inserted using parameter binding
    And Query "SELECT * FROM {table} ORDER BY C1 NULLS LAST" is executed
    Then the result should contain the bound INTERVAL DAY TO SECOND values ['-99999 23:59:59.999999', '-1 2:3:4.567', '0 0:0:0.0', '12 3:4:5.678', '99999 23:59:59.999999', NULL]

  @odbc_e2e
  Scenario: should select INTERVAL YEAR TO MONTH values using parameter binding
    Given Snowflake client is logged in
    When Query "SELECT ?::INTERVAL YEAR TO MONTH, ?::INTERVAL YEAR TO MONTH, ?::INTERVAL YEAR TO MONTH" is executed with bound string values ['0-0', '1-2', '999999999-11']
    Then the result should contain:
      | col1 | col2 | col3           |
      | 0-0  | 1-2  | 999999999-11   |

  @odbc_e2e
  Scenario: should select INTERVAL DAY TO SECOND values using parameter binding
    Given Snowflake client is logged in
    When Query "SELECT ?::INTERVAL DAY TO SECOND, ?::INTERVAL DAY TO SECOND, ?::INTERVAL DAY TO SECOND" is executed with bound string values ['0 0:0:0.0', '12 3:4:5.678', '99999 23:59:59.999999']
    Then the result should contain:
      | col1           | col2           | col3                        |
      | 0 0:0:0.0      | 12 3:4:5.678   | 99999 23:59:59.999999       |

  @odbc_e2e
  Scenario: should select NULL INTERVAL values using parameter binding
    Given Snowflake client is logged in
    When Query "SELECT ?::INTERVAL YEAR TO MONTH, ?::INTERVAL DAY TO SECOND" is executed with bound NULL values
    Then the result should contain:
      | col1 | col2 |
      | NULL | NULL |

  @odbc_e2e
  Scenario: should select INTERVAL YEAR values using parameter binding
    Given Snowflake client is logged in
    When Query "SELECT ?::INTERVAL YEAR, ?::INTERVAL YEAR, ?::INTERVAL YEAR" is executed with bound string values ['0', '2', '-999999999']
    Then the result should contain expected INTERVAL YEAR bound values in order

  @odbc_e2e
  Scenario: should select INTERVAL MONTH values using parameter binding
    Given Snowflake client is logged in
    When Query "SELECT ?::INTERVAL MONTH, ?::INTERVAL MONTH, ?::INTERVAL MONTH" is executed with bound string values ['0', '5', '-999999999']
    Then the result should contain expected INTERVAL MONTH bound values in order

  @odbc_e2e
  Scenario: should select INTERVAL DAY values using parameter binding
    Given Snowflake client is logged in
    When Query "SELECT ?::INTERVAL DAY, ?::INTERVAL DAY, ?::INTERVAL DAY" is executed with bound string values ['0', '1', '-999999999']
    Then the result should contain expected INTERVAL DAY bound values in order

  @odbc_e2e
  Scenario: should select INTERVAL HOUR values using parameter binding
    Given Snowflake client is logged in
    When Query "SELECT ?::INTERVAL HOUR, ?::INTERVAL HOUR, ?::INTERVAL HOUR" is executed with bound string values ['0', '5', '-999999999']
    Then the result should contain expected INTERVAL HOUR bound values in order

  @odbc_e2e
  Scenario: should select INTERVAL MINUTE values using parameter binding
    Given Snowflake client is logged in
    When Query "SELECT ?::INTERVAL MINUTE, ?::INTERVAL MINUTE, ?::INTERVAL MINUTE" is executed with bound string values ['0', '4', '-999999999']
    Then the result should contain expected INTERVAL MINUTE bound values in order

  @odbc_e2e
  Scenario: should select INTERVAL SECOND values using parameter binding
    Given Snowflake client is logged in
    When Query "SELECT ?::INTERVAL SECOND, ?::INTERVAL SECOND, ?::INTERVAL SECOND" is executed with bound string values ['0', '8.5', '-999999999.999999']
    Then the result should contain expected INTERVAL SECOND bound values in order

  @odbc_e2e
  Scenario: should select INTERVAL DAY TO HOUR values using parameter binding
    Given Snowflake client is logged in
    When Query "SELECT ?::INTERVAL DAY TO HOUR, ?::INTERVAL DAY TO HOUR" is executed with bound string values ['1 2', '-999999999 23']
    Then the result should contain expected INTERVAL DAY TO HOUR bound values in order

  @odbc_e2e
  Scenario: should select INTERVAL DAY TO MINUTE values using parameter binding
    Given Snowflake client is logged in
    When Query "SELECT ?::INTERVAL DAY TO MINUTE, ?::INTERVAL DAY TO MINUTE" is executed with bound string values ['1 2:30', '-999999999 23:59']
    Then the result should contain expected INTERVAL DAY TO MINUTE bound values in order

  @odbc_e2e
  Scenario: should select INTERVAL HOUR TO MINUTE values using parameter binding
    Given Snowflake client is logged in
    When Query "SELECT ?::INTERVAL HOUR TO MINUTE, ?::INTERVAL HOUR TO MINUTE" is executed with bound string values ['1:30', '-999999999:59']
    Then the result should contain expected INTERVAL HOUR TO MINUTE bound values in order

  @odbc_e2e
  Scenario: should select INTERVAL HOUR TO SECOND values using parameter binding
    Given Snowflake client is logged in
    When Query "SELECT ?::INTERVAL HOUR TO SECOND, ?::INTERVAL HOUR TO SECOND" is executed with bound string values ['1:30:45.123', '-999999999:59:59.999999']
    Then the result should contain expected INTERVAL HOUR TO SECOND bound values in order

  @odbc_e2e
  Scenario: should select INTERVAL MINUTE TO SECOND values using parameter binding
    Given Snowflake client is logged in
    When Query "SELECT ?::INTERVAL MINUTE TO SECOND, ?::INTERVAL MINUTE TO SECOND" is executed with bound string values ['30:45.123', '-999999999:59.999999']
    Then the result should contain expected INTERVAL MINUTE TO SECOND bound values in order

  # ============================================================================
  # MULTIPLE CHUNKS DOWNLOADING
  # ============================================================================

  @odbc_e2e
  Scenario: should download INTERVAL YEAR TO MONTH data in multiple chunks
    # ~50000 values ensures data is downloaded in at least two chunks
    Given Snowflake client is logged in
    When Query "SELECT '0-1'::INTERVAL YEAR TO MONTH * SEQ4() AS ym FROM TABLE(GENERATOR(ROWCOUNT => 50000)) v ORDER BY ym" is executed
    Then there are 50000 rows returned
    And all returned INTERVAL YEAR TO MONTH values should form a sequential series of months starting at 0

  @odbc_e2e
  Scenario: should download INTERVAL DAY TO SECOND data in multiple chunks
    # ~50000 values ensures data is downloaded in at least two chunks
    Given Snowflake client is logged in
    When Query "SELECT '0 0:0:1.0'::INTERVAL DAY TO SECOND * SEQ4() AS dt FROM TABLE(GENERATOR(ROWCOUNT => 50000)) v ORDER BY dt" is executed
    Then there are 50000 rows returned
    And all returned INTERVAL DAY TO SECOND values should form a sequential series of seconds starting at 0

  # ============================================================================
  # INTERVAL ARITHMETIC
  # ============================================================================

  @odbc_e2e
  Scenario: should respect order of interval components in date arithmetic
    # Docs show that INTERVAL '1 day, 1 year' vs '1 year, 1 day' give different results:
    #   TO_DATE('2019-02-28') + INTERVAL '1 day, 1 year'  -> 2020-03-01
    #   TO_DATE('2019-02-28') + INTERVAL '1 year, 1 day'  -> 2020-02-29
    Given Snowflake client is logged in
    When Query "SELECT TO_DATE('2019-02-28') + INTERVAL '1 day, 1 year' AS d1, TO_DATE('2019-02-28') + INTERVAL '1 year, 1 day' AS d2" is executed
    Then the result should contain:
      | d1        | d2        |
      | 2020-03-01 | 2020-02-29 |

  @odbc_e2e
  Scenario: should support complex INTERVAL with mixed units and abbreviations
    # Docs example (abbreviated parts): TO_DATE('2025-01-17') +
    #   INTERVAL '1 y, 3 q, 4 mm, 5 w, 6 d, 7 h, 9 m, 8 s,
    #             1000 ms, 445343232 us, 898498273498 ns'
    # -> 2027-03-30 07:31:32.841505498
    Given Snowflake client is logged in
    When Query "SELECT TO_DATE('2025-01-17') + INTERVAL '1 y, 3 q, 4 mm, 5 w, 6 d, 7 h, 9 m, 8 s, 1000 ms, 445343232 us, 898498273498 ns' AS complex_interval" is executed
    Then the result should contain:
      | complex_interval              |
      | 2027-03-30 07:31:32.841505498 |

  @odbc_e2e
  Scenario: should add two INTERVAL YEAR TO MONTH values
    # Interval + Interval returns an interval column, not a date/time
    Given Snowflake client is logged in
    When Query "SELECT '1-2'::INTERVAL YEAR TO MONTH + '0-3'::INTERVAL YEAR TO MONTH AS i" is executed
    Then the result should contain expected INTERVAL YEAR TO MONTH value '1-5'

  @odbc_e2e
  Scenario: should add two INTERVAL DAY TO SECOND values
    Given Snowflake client is logged in
    When Query "SELECT '1 2:30:00.0'::INTERVAL DAY TO SECOND + '0 1:45:30.5'::INTERVAL DAY TO SECOND AS i" is executed
    Then the result should contain expected INTERVAL DAY TO SECOND value '1 4:15:30.500000'

  @odbc_e2e
  Scenario: should negate an INTERVAL value
    Given Snowflake client is logged in
    When Query "SELECT -('1-6'::INTERVAL YEAR TO MONTH) AS ym, -('3 12:0:0.0'::INTERVAL DAY TO SECOND) AS dt" is executed
    Then the result should contain expected negated INTERVAL values '-1-6' and '-3 12:0:0.000000'

  @odbc_e2e
  Scenario: should subtract two INTERVAL values
    Given Snowflake client is logged in
    When Query "SELECT '1-5'::INTERVAL YEAR TO MONTH - '0-3'::INTERVAL YEAR TO MONTH AS ym, '1 4:15:30.5'::INTERVAL DAY TO SECOND - '0 1:45:30.5'::INTERVAL DAY TO SECOND AS dt" is executed
    Then the result should contain expected INTERVAL values '1-2' and '1 2:30:00.000000'

  @odbc_e2e
  Scenario: should multiply INTERVAL by a scalar
    Given Snowflake client is logged in
    When Query "SELECT '0-6'::INTERVAL YEAR TO MONTH * 3 AS ym, 2 * '1 0:0:0.0'::INTERVAL DAY TO SECOND AS dt" is executed
    Then the result should contain expected INTERVAL values '1-6' and '2 0:0:0.000000'

  @odbc_e2e
  Scenario: should divide INTERVAL by a scalar
    Given Snowflake client is logged in
    When Query "SELECT '1-6'::INTERVAL YEAR TO MONTH / 3 AS ym, '2 0:0:0.0'::INTERVAL DAY TO SECOND / 2 AS dt" is executed
    Then the result should contain expected INTERVAL values '0-6' and '1 0:0:0.000000'
