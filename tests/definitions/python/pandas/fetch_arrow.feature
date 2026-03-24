@python
Feature: Arrow fetch methods (Python-specific)

  # =========================================================================== #
  #                         fetch_arrow_all                                     #
  # =========================================================================== #

  @python_e2e
  Scenario Outline: should fetch <type_name> with null as pyarrow Table
    Given Snowflake client is logged in
    And Query "ALTER SESSION SET TIMEZONE = 'UTC'" is executed
    When Query "SELECT <value_expr> AS val, <null_expr> AS null_val" is executed
    And fetch_arrow_all is called
    Then The result should be a pyarrow.Table with 1 row
    And Column VAL should have the correct value for <type_name>
    And Column NULL_VAL should be null

    Examples:
      | type_name     | value_expr                                   | null_expr           |
      | number        | 1::NUMBER                                    | NULL::NUMBER        |
      | scaled_number | 3.14::NUMBER(10,2)                           | NULL::NUMBER(10,2)  |
      | varchar       | 'hello'::VARCHAR                             | NULL::VARCHAR       |
      | float         | 1.5::FLOAT                                   | NULL::FLOAT         |
      | boolean       | TRUE::BOOLEAN                                | NULL::BOOLEAN       |
      | date          | '2026-03-23'::DATE                           | NULL::DATE          |
      | time          | '12:30:00'::TIME                             | NULL::TIME          |
      | timestamp_ntz | '2026-03-23 10:30:00'::TIMESTAMP_NTZ         | NULL::TIMESTAMP_NTZ |
      | timestamp_ltz | '2026-03-23 10:30:00'::TIMESTAMP_LTZ         | NULL::TIMESTAMP_LTZ |
      | timestamp_tz  | '2026-03-23 10:30:00 +0530'::TIMESTAMP_TZ    | NULL::TIMESTAMP_TZ  |
      | binary        | TO_BINARY('ABCD','HEX')::BINARY              | NULL::BINARY        |
      | variant       | TO_VARIANT(42)                               | NULL::VARIANT       |
      | array         | ARRAY_CONSTRUCT(1,2,3)::ARRAY                | NULL::ARRAY         |
      | object        | OBJECT_CONSTRUCT('key','value')::OBJECT      | NULL::OBJECT        |

  @python_e2e
  Scenario: should return None from fetch_arrow_all for empty result set
    Given Snowflake client is logged in
    When Query "SELECT 1 AS id WHERE 1=0" is executed
    And fetch_arrow_all is called
    Then The result should be None

  @python_e2e
  Scenario Outline: should return empty <type_name> column with correct Arrow type when force_return_table is true
    Given Snowflake client is logged in
    When Query "SELECT <value_expr> AS col WHERE 1=0" is executed
    And fetch_arrow_all is called with force_return_table=True
    Then The result should be a pyarrow.Table with 0 rows
    And Column COL should have <arrow_type> Arrow type

    Examples:
      | type_name     | value_expr                                   | arrow_type |
      | number        | 1::NUMBER                                    | int64      |
      | scaled_number | 3.14::NUMBER(10,2)                           | int64      |
      | varchar       | 'hello'::VARCHAR                             | string     |
      | float         | 1.5::FLOAT                                   | float64    |
      | boolean       | TRUE::BOOLEAN                                | boolean    |
      | date          | '2026-03-23'::DATE                           | date       |
      | time          | '12:30:00'::TIME                             | time       |
      | timestamp_ntz | '2026-03-23 10:30:00'::TIMESTAMP_NTZ         | timestamp  |
      | timestamp_ltz | '2026-03-23 10:30:00'::TIMESTAMP_LTZ         | timestamp  |
      | timestamp_tz  | '2026-03-23 10:30:00 +0530'::TIMESTAMP_TZ    | timestamp  |
      | binary        | TO_BINARY('ABCD','HEX')::BINARY              | binary     |
      | variant       | TO_VARIANT(42)                               | string     |
      | array         | ARRAY_CONSTRUCT(1,2,3)::ARRAY                | string     |
      | object        | OBJECT_CONSTRUCT('key','value')::OBJECT      | string     |

  @python_e2e
  Scenario: should convert scaled fixed number to decimal via fetch_arrow_all
    Given Snowflake client is logged in
    And arrow_number_to_decimal is set to True on the connection
    When Query "SELECT 3.14::NUMBER(10,2) AS pi" is executed
    And fetch_arrow_all is called
    Then Column PI should be Decimal128

  @python_e2e
  Scenario: should force microsecond precision for timestamps via fetch_arrow_all
    Given Snowflake client is logged in
    When Query "SELECT '2024-01-15 10:30:00.123456789'::TIMESTAMP_NTZ(9) AS ts" is executed
    And fetch_arrow_all is called with force_microsecond_precision=True
    Then Column TS should be a timestamp type with microsecond unit
    And Column TS value should have microsecond=123456

  # =========================================================================== #
  #                        fetch_arrow_batches                                  #
  # =========================================================================== #

  @python_e2e
  Scenario: should yield multiple arrow batches for large result set
    Given Snowflake client is logged in
    When Query "SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 100000)) v" is executed
    And fetch_arrow_batches is called
    Then More than one batch should be yielded
    And Each element should be a pyarrow.Table
    And The total row count across all batches should be 100000
