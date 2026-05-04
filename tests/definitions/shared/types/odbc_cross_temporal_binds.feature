@odbc @core_not_needed @jdbc_not_needed @python_not_needed
Feature: ODBC cross-temporal SQLBindParameter conversions

  ODBC-only behavior: SQLBindParameter lets the application bind one
  C temporal type (SQL_C_TYPE_DATE / TIME / TIMESTAMP) to a *different*
  SQL temporal target type. Per the ODBC Programmer's Reference,
  Appendix D ("Converting Data from C to SQL Data Types"), every
  cross-temporal pair has well-defined conversion semantics — including
  spec-mandated SQLSTATE diagnostics when a struct field is out of range
  (22007 "Invalid datetime format") or when a narrowing conversion would
  silently lose information (22008 "Datetime field overflow"). JDBC and
  Python expose a single date/time/datetime type per direction, so they
  cannot exercise this matrix; this feature is owned exclusively by the
  ODBC driver surface.
  Reference:
    https://learn.microsoft.com/sql/odbc/reference/appendixes/converting-data-from-c-to-sql-data-types

  # =========================================================================== #
  #                  Happy paths — Appendix D conversion rules                  #
  # =========================================================================== #

  @odbc_e2e
  Scenario Outline: should bind SQL_C_TYPE_DATE to SQL_TYPE_TIMESTAMP and zero the time portion
    # Appendix D, "C to SQL: Date" — the time fields of the timestamp must
    # be set to 00:00:00.000000000 regardless of the Snowflake variant
    # (NTZ / LTZ / TZ all flow through the same C-side conversion).
    Given Snowflake client is logged in
    And Session TIMEZONE is set to UTC
    And Table with <variant> column exists
    When SQL_C_TYPE_DATE 2026-04-13 is bound to SQL_TYPE_TIMESTAMP and inserted via SQLBindParameter
    Then the stored timestamp date matches 2026-04-13
    And the stored timestamp time portion is exactly 00:00:00.000000000

    Examples:
      | variant       |
      | TIMESTAMP_NTZ |
      | TIMESTAMP_LTZ |
      | TIMESTAMP_TZ  |

  @odbc_e2e
  Scenario Outline: should bind SQL_C_TYPE_TIME to SQL_TYPE_TIMESTAMP using current local date and zero fraction
    # Appendix D, "C to SQL: Time" — the date fields are set to the current
    # local date and the fractional-seconds portion is set to zero.
    Given Snowflake client is logged in
    And Session TIMEZONE is set to UTC
    And Table with <variant> column exists
    When SQL_C_TYPE_TIME 14:30:45 is bound to SQL_TYPE_TIMESTAMP and inserted via SQLBindParameter
    Then the stored timestamp time matches 14:30:45
    And the stored timestamp fraction is zero
    And the stored timestamp date is the local date at bind time

    Examples:
      | variant       |
      | TIMESTAMP_NTZ |
      | TIMESTAMP_LTZ |
      | TIMESTAMP_TZ  |

  @odbc_e2e
  Scenario: should bind SQL_C_TYPE_TIMESTAMP to SQL_TYPE_DATE and discard the zero time portion
    # Appendix D, "C to SQL: Timestamp" — when bound to a DATE target the
    # time fields of the source struct must be exactly zero; the date
    # fields are stored verbatim.
    Given Snowflake client is logged in
    And Table with DATE column exists
    When SQL_C_TYPE_TIMESTAMP 2026-04-13 00:00:00.000000000 is bound to SQL_TYPE_DATE and inserted via SQLBindParameter
    Then the stored date matches 2026-04-13

  @odbc_e2e
  Scenario: should bind SQL_C_TYPE_TIMESTAMP to SQL_TYPE_TIME and discard the date portion
    # Appendix D, "C to SQL: Timestamp" — when bound to a TIME target the
    # fractional-seconds portion of the source struct must be exactly
    # zero; the date fields are silently discarded.
    Given Snowflake client is logged in
    And Table with TIME column exists
    When SQL_C_TYPE_TIMESTAMP 2026-04-13 14:30:45.000000000 is bound to SQL_TYPE_TIME and inserted via SQLBindParameter
    Then the stored time matches 14:30:45

  # =========================================================================== #
  #          Datetime field overflow — SQLSTATE 22008 (silent narrowing)        #
  # =========================================================================== #

  @odbc_e2e
  Scenario Outline: should reject TIMESTAMP→DATE with non-zero <field> with SQLSTATE 22008
    # Appendix D mandates SQL_ERROR with SQLSTATE 22008 ("Datetime field
    # overflow") when binding a SQL_C_TYPE_TIMESTAMP whose discarded time
    # portion is non-zero — the driver must NOT silently truncate.
    Given Snowflake client is logged in
    And Table with DATE column exists
    When SQL_C_TYPE_TIMESTAMP <ts> is bound to SQL_TYPE_DATE and SQLExecute is called
    Then SQLExecute fails with SQLSTATE 22008

    Examples:
      | field    | ts                            |
      | hour     | 2026-04-13 14:00:00.000000000 |
      | minute   | 2026-04-13 00:30:00.000000000 |
      | second   | 2026-04-13 00:00:45.000000000 |
      | fraction | 2026-04-13 00:00:00.500000000 |

  @odbc_e2e
  Scenario: should reject TIMESTAMP→TIME with non-zero fraction with SQLSTATE 22008
    # Whole-second h/m/s round-trip cleanly; only the fractional part
    # would be silently dropped, so the spec requires SQL_ERROR / 22008.
    Given Snowflake client is logged in
    And Table with TIME column exists
    When SQL_C_TYPE_TIMESTAMP 2026-04-13 14:30:45.500000000 is bound to SQL_TYPE_TIME and SQLExecute is called
    Then SQLExecute fails with SQLSTATE 22008

  # =========================================================================== #
  #         Invalid struct fields — SQLSTATE 22007 (Invalid datetime format)    #
  # =========================================================================== #
  # 22007 takes precedence over 22008: when a struct field is itself out
  # of legal range (e.g. month=13, hour=25), the driver must surface the
  # struct-validity error regardless of whether a narrowing 22008 would
  # otherwise apply.

  @odbc_e2e
  Scenario Outline: should reject SQL_C_TYPE_DATE with invalid <field> bound to SQL_TYPE_TIMESTAMP with SQLSTATE 22007
    Given Snowflake client is logged in
    And Table with TIMESTAMP_NTZ column exists
    When SQL_C_TYPE_DATE with <field>=<bad_value> is bound to SQL_TYPE_TIMESTAMP and SQLExecute is called
    Then SQLExecute fails with SQLSTATE 22007

    Examples:
      | field | bad_value |
      | month | 13        |
      | month | 0         |
      | day   | 32        |
      | day   | 0         |

  @odbc_e2e
  Scenario Outline: should reject SQL_C_TYPE_TIME with invalid <field> bound to SQL_TYPE_TIMESTAMP with SQLSTATE 22007
    Given Snowflake client is logged in
    And Table with TIMESTAMP_NTZ column exists
    When SQL_C_TYPE_TIME with <field>=<bad_value> is bound to SQL_TYPE_TIMESTAMP and SQLExecute is called
    Then SQLExecute fails with SQLSTATE 22007

    Examples:
      | field  | bad_value |
      | hour   | 24        |
      | minute | 60        |
      | second | 60        |

  @odbc_e2e
  Scenario Outline: should reject SQL_C_TYPE_TIMESTAMP with invalid <field> bound to SQL_TYPE_DATE with SQLSTATE 22007
    Given Snowflake client is logged in
    And Table with DATE column exists
    When SQL_C_TYPE_TIMESTAMP with <field>=<bad_value> is bound to SQL_TYPE_DATE and SQLExecute is called
    Then SQLExecute fails with SQLSTATE 22007

    Examples:
      | field | bad_value |
      | month | 13        |
      | day   | 32        |

  @odbc_e2e
  Scenario Outline: should reject SQL_C_TYPE_TIMESTAMP with invalid <field> bound to SQL_TYPE_TIME with SQLSTATE 22007
    Given Snowflake client is logged in
    And Table with TIME column exists
    When SQL_C_TYPE_TIMESTAMP with <field>=<bad_value> is bound to SQL_TYPE_TIME and SQLExecute is called
    Then SQLExecute fails with SQLSTATE 22007

    Examples:
      | field    | bad_value     |
      | hour     | 24            |
      | minute   | 60            |
      | fraction | 3000000000    |
