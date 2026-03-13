@python @odbc @jdbc @core_not_needed
Feature: DECFLOAT type support

  # =========================================================================== #
  #                               Type casting                                  #
  # =========================================================================== #

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should cast decfloat values to appropriate type
    # Python: Values should be cast to 'Decimal' type with 38-digit precision
    Given Snowflake client is logged in
    When Query "SELECT 0::DECFLOAT, 123.456::DECFLOAT, 1.23e37::DECFLOAT, '12345678901234567890123456789012345678'::DECFLOAT" is executed
    Then All values should be returned as appropriate type
    And Values should maintain full 38-digit precision

  # =========================================================================== #
  #                     SELECT with literals (no tables)                        #
  # =========================================================================== #

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should select decfloat literals
    Given Snowflake client is logged in
    When Query "SELECT 0::DECFLOAT, 1.5::DECFLOAT, -1.5::DECFLOAT, 123.456789::DECFLOAT, -987.654321::DECFLOAT" is executed
    Then Result should contain exact decimals [0, 1.5, -1.5, 123.456789, -987.654321]

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should handle full 38-digit precision values from literals
    Given Snowflake client is logged in
    When Query "SELECT '12345678901234567890123456789012345678'::DECFLOAT, '1.2345678901234567890123456789012345678E+100'::DECFLOAT, '1.2345678901234567890123456789012345678E-100'::DECFLOAT" is executed
    Then Result should preserve all 38 digits for each value

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should handle extreme exponent values from literals
    Given Snowflake client is logged in
    When Query "SELECT '1E+16384'::DECFLOAT, '1E-16383'::DECFLOAT" is executed
    Then Result should contain [1E+16384, 1E-16383]
    When Query "SELECT '-1.234E+8000'::DECFLOAT, '9.876E-8000'::DECFLOAT" is executed
    Then Result should contain [-1.234E+8000, 9.876E-8000]

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should handle NULL values from literals
    Given Snowflake client is logged in
    When Query "SELECT NULL::DECFLOAT, 42.5::DECFLOAT, NULL::DECFLOAT" is executed
    Then Result should contain [NULL, 42.5, NULL]

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should download large result set with multiple chunks from GENERATOR
    Given Snowflake client is logged in
    When Query "SELECT seq8()::DECFLOAT as id FROM TABLE(GENERATOR(ROWCOUNT => 20000)) v" is executed
    Then Result should contain consecutive numbers from 0 to 19999 returned as appropriate type

  # =========================================================================== #
  #                             Table operations                                #
  # =========================================================================== #

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should select decfloats from table
    Given Snowflake client is logged in
    And Table with DECFLOAT column exists with values [0, 123.456, -789.012, 1.23e20, -9.87e-15]
    When Query "SELECT * FROM <table>" is executed
    Then Result should contain exact decimals [0, 123.456, -789.012, 1.23e20, -9.87e-15]

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should handle full 38-digit precision values from table
    Given Snowflake client is logged in
    And Table with DECFLOAT column exists with values [12345678901234567890123456789012345678, 1.2345678901234567890123456789012345678E+100, 1.2345678901234567890123456789012345678E-100]
    When Query "SELECT * FROM <table>" is executed
    Then Result should preserve all 38 digits for each value

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should handle extreme exponent values from table
    Given Snowflake client is logged in
    And Table with DECFLOAT column exists with values [1E+16384, 1E-16383, -1.234E+8000, 9.876E-8000]
    When Query "SELECT * FROM <table>" is executed
    Then Result should contain [1E+16384, 1E-16383, -1.234E+8000, 9.876E-8000]

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should handle NULL values from table
    Given Snowflake client is logged in
    And Table with DECFLOAT column exists with values [NULL, 123.456, NULL, -789.012]
    When Query "SELECT * FROM <table>" is executed
    Then Result should contain [NULL, 123.456, NULL, -789.012]

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should download large result set with multiple chunks from table
    Given Snowflake client is logged in
    And Table with DECFLOAT column exists with values from 0 to 19999
    When Query "SELECT * FROM <table>" is executed
    Then Result should contain consecutive numbers from 0 to 19999 returned as appropriate type

  # =========================================================================== #
  #                            Parameter binding                                #
  # =========================================================================== #

  @python_e2e @jdbc_e2e @odbc_e2e
  Scenario: should select decfloat using parameter binding
    Given Snowflake client is logged in
    When Query "SELECT ?::DECFLOAT, ?::DECFLOAT, ?::DECFLOAT" is executed with bound DECFLOAT values [123.456, -789.012, 42.0]
    Then Result should contain [123.456, -789.012, 42.0]
    When Query "SELECT ?::DECFLOAT" is executed with bound NULL value
    Then Result should contain [NULL]

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should select extreme decfloat values using parameter binding
    Given Snowflake client is logged in
    When Query "SELECT ?::DECFLOAT" is executed with bound value 1E+16384
    Then Result should contain [1E+16384]
    When Query "SELECT ?::DECFLOAT" is executed with bound value -1.234E+8000
    Then Result should contain [-1.234E+8000]

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should insert decfloat using parameter binding
    Given Snowflake client is logged in
    And Table with DECFLOAT column exists
    When DECFLOAT values [0, 123.456, -789.012, NULL] are inserted using explicit binding
    Then SELECT should return the same exact values

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should insert extreme decfloat values using parameter binding
    Given Snowflake client is logged in
    And Table with DECFLOAT column exists
    When DECFLOAT values [1E+16384, 1E-16383, -1.234E+8000] are inserted using explicit binding
    And Query "SELECT * FROM <table>" is executed
    Then SELECT should return the same exact values
