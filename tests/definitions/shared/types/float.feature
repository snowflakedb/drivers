@python @odbc @jdbc @core_not_needed
Feature: FLOAT type support

  # =========================================================================== #
  #                               Type casting                                  #
  # =========================================================================== #

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should cast float values to appropriate type for float and synonyms
    # Python: Values should be cast to 'float' type (64-bit)
    Given Snowflake client is logged in
    When Query "SELECT 0.0::<type>, 123.456::<type>, 1.23e10::<type>, 'NaN'::<type>, 'inf'::<type>" is executed
    Then All values should be returned as appropriate type
    And Regular values should have approximately 15 decimal digits precision
    And NaN and inf values should be identified correctly

  # =========================================================================== #
  #                     SELECT with literals (no tables)                        #
  # =========================================================================== #

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should select float literals for float and synonyms
    Given Snowflake client is logged in
    When Query "SELECT 0.0::<type>, 1.0::<type>, -1.0::<type>, 123.456::<type>, -123.456::<type>" is executed
    Then Result should contain floats [0.0, 1.0, -1.0, 123.456, -123.456]

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should handle special float values from literals for float and synonyms
    Given Snowflake client is logged in
    When Query "SELECT 'NaN'::<type>, 'inf'::<type>, '-inf'::<type>" is executed
    Then Result should contain [NaN, positive_infinity, negative_infinity]

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario Outline: should handle float <case> boundary values from literals for float and synonyms
    Given Snowflake client is logged in
    When Query "SELECT <query_values>" is executed
    Then Result should contain floats [<expected_values>]

    Examples:
      | case | query_values                                                   | expected_values                                 |
      | max  | 1.7976931348623157e308::<type>, -1.7976931348623157e308::<type> | 1.7976931348623157e308, -1.7976931348623157e308  |
      | min  | 2.2250738585072014e-308::<type>, 5e-324::<type>                | 2.2250738585072014e-308, approximately 5e-324    |

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should handle float precision boundary values from literals for float and synonyms
    Given Snowflake client is logged in
    When Query "SELECT 123456789012345.0::<type>, 1234567890123456.0::<type>" is executed
    Then Result should verify precision around 15 decimal digits

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should handle NULL values from literals for float and synonyms
    Given Snowflake client is logged in
    When Query "SELECT NULL::<type>, 42.5::<type>, NULL::<type>" is executed
    Then Result should contain [NULL, 42.5, NULL]

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should download large result set with multiple chunks from GENERATOR for float and synonyms
    Given Snowflake client is logged in
    When Query "SELECT seq8()::<type> as id FROM TABLE(GENERATOR(ROWCOUNT => 50000)) v" is executed
    Then Result should contain 50000 rows with all values returned as appropriate float type

  # =========================================================================== #
  #                             Table operations                                #
  # =========================================================================== #

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should select floats from table for float and synonyms
    Given Snowflake client is logged in
    And Table with <type> column exists with values [0.0, 123.456, -789.012, 1.23e5, -9.87e-3]
    When Query "SELECT * FROM float_table" is executed
    Then Result should contain floats [0.0, 123.456, -789.012, 123000.0, -0.00987]

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should handle special float values from table for float and synonyms
    Given Snowflake client is logged in
    And Table with <type> column exists with values [NaN, inf, -inf, 42.0, -42.0]
    When Query "SELECT * FROM <table>" is executed
    Then Result should contain [NaN, positive_infinity, negative_infinity, 42.0, -42.0]

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should handle float boundary values from table for float and synonyms
    Given Snowflake client is logged in
    And Table with <type> column exists with boundary values [1.7976931348623157e308, -1.7976931348623157e308, 2.2250738585072014e-308, 5e-324, 123456789012345.0]
    When Query "SELECT * FROM <table>" is executed
    Then Result should contain maximum, minimum, and precision boundary values preserved within float precision limits

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should handle NULL values from table for float and synonyms
    Given Snowflake client is logged in
    And Table with <type> column exists with values [NULL, 123.456, NULL, -789.012]
    When Query "SELECT * FROM <table>" is executed
    Then Result should contain [NULL, 123.456, NULL, -789.012]

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should select large result set from table for float and synonyms
    Given Snowflake client is logged in
    And Table with <type> column exists with 50000 sequential values
    When Query "SELECT * FROM <table>" is executed
    Then Result should contain 50000 rows with all values returned as appropriate float type

  # =========================================================================== #
  #                            Parameter binding                                #
  # =========================================================================== #

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should select float using parameter binding for float and synonyms
    Given Snowflake client is logged in
    When Query "SELECT ?::<type>, ?::<type>, ?::<type>" is executed with bound float values [123.456, -789.012, 42.0]
    Then Result should contain floats [123.456, -789.012, 42.0]

  @python_e2e @jdbc_e2e
  Scenario: should select null float using parameter binding for float and synonyms
    Given Snowflake client is logged in
    When Query "SELECT ?::<type>" is executed with bound NULL value
    Then Result should contain NULL

  @python_e2e @odbc_e2e
  Scenario: should insert float using parameter binding for float and synonyms
    Given Snowflake client is logged in
    And Table with <type> column exists
    When Float values [0.0, 123.456, -789.012, NULL] are bulk-inserted using multirow binding
    # Note: NaN, inf, -inf cannot be bound — Snowflake rejects them as bind values.
    Then Result should contain the same values including NULL
