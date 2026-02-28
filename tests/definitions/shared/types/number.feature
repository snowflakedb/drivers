@python @odbc @jdbc @core_not_needed
Feature: NUMBER type support

  # =========================================================================== #
  #                               Type casting                                  #
  # =========================================================================== #

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should cast number values to appropriate type for number and synonyms
    # Python: scale=0 → int, scale>0 → Decimal
    Given Snowflake client is logged in
    When Query "SELECT 0::<type>(10,0), 123::<type>(10,0), 0.00::<type>(10,2), 123.45::<type>(10,2)" is executed
    Then All values should be returned as appropriate type
    And Values should match [0, 123, 0.00, 123.45]

  # =========================================================================== #
  #                     SELECT with literals (no tables)                        #
  # =========================================================================== #

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should select number literals for number and synonyms
    Given Snowflake client is logged in
    When Query "SELECT 0::<type>(10,0), -456::<type>(10,0), 1.50::<type>(10,2), -123.45::<type>(10,2), 123.456::<type>(15,3), -789.012::<type>(15,3)" is executed
    Then Result should contain [0, -456, 1.50, -123.45, 123.456, -789.012]

  @python_e2e @jdbc_e2e
  Scenario: should handle high precision values from literals for number and synonyms
    Given Snowflake client is logged in
    When Query "SELECT 12345678901234567890123456789012345678::<type>(38,0), 123456789012345678901234567890123456.78::<type>(38,2), 1234567890123456789012345678.1234567890::<type>(38,10), 0.0000000000000000000000000000000000001::<type>(38,37)" is executed
    Then Result should contain [12345678901234567890123456789012345678, 123456789012345678901234567890123456.78, 1234567890123456789012345678.1234567890, 0.0000000000000000000000000000000000001]

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should handle scale and precision boundaries from literals for number and synonyms
    Given Snowflake client is logged in
    When Query "SELECT 999.99::<type>(5,2), -999.99::<type>(5,2), 99999999::<type>(8,0), -99999999::<type>(8,0)" is executed
    Then Result should contain [999.99, -999.99, 99999999, -99999999]

  @python_e2e @jdbc_e2e
  Scenario: should handle high precision boundaries from literals for number and synonyms
    Given Snowflake client is logged in
    When Query "SELECT 99999999999999999999999999999999999999::<type>(38,0), -99999999999999999999999999999999999999::<type>(38,0)" is executed
    Then Result should contain max and min 38-digit integers

  @python_e2e @jdbc_e2e
  Scenario: should handle NULL values from literals for number and synonyms
    Given Snowflake client is logged in
    When Query "SELECT NULL::<type>(10,0), 42::<type>(10,0), NULL::<type>(10,2), 42.50::<type>(10,2)" is executed
    Then Result should contain [NULL, 42, NULL, 42.50]

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should download large result set with multiple chunks from GENERATOR for number and synonyms
    Given Snowflake client is logged in
    When Query "SELECT seq8()::<type>(38,0), (seq8() + 0.12345)::<type>(20,5) FROM TABLE(GENERATOR(ROWCOUNT => 30000)) v" is executed
    Then Column 1 should contain sequential integers from 0 to 29999
    And Column 2 should contain sequential decimals starting from 0.12345

  # =========================================================================== #
  #                             Table operations                                #
  # =========================================================================== #

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should select numbers from table with multiple scales for number and synonyms
    Given Snowflake client is logged in
    And Table with columns (<type>(10,0), <type>(10,2), <type>(15,3), <type>(20,5)) exists
    And Row (123, 123.45, 123.456, 12345.67890) is inserted
    And Row (-456, -67.89, -789.012, -98765.43210) is inserted
    And Row (0, 0.00, 0.000, 0.00000) is inserted
    And Row (999999, 999.99, 1000.500, 123456.78901) is inserted
    When Query "SELECT * FROM <table>" is executed
    Then Result should contain 4 rows with expected values

  @python_e2e @jdbc_e2e
  Scenario: should handle high precision values from table for number and synonyms
    Given Snowflake client is logged in
    And Table with columns (<type>(38,0), <type>(38,2), <type>(38,10), <type>(38,37)) exists
    And Row (12345678901234567890123456789012345678, 123456789012345678901234567890123456.78, 1234567890123456789012345678.1234567890, 1.2345678901234567890123456789012345678) is inserted
    When Query "SELECT * FROM <table>" is executed
    Then Result should contain [12345678901234567890123456789012345678, 123456789012345678901234567890123456.78, 1234567890123456789012345678.1234567890, 1.2345678901234567890123456789012345678]

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should handle scale and precision boundaries from table for number and synonyms
    Given Snowflake client is logged in
    And Table with columns (<type>(5,2), <type>(8,0)) exists
    And Row (999.99, 99999999) is inserted
    And Row (-999.99, -99999999) is inserted
    And Row (123.45, 12345678) is inserted
    And Row (0.01, 0) is inserted
    When Query "SELECT * FROM <table>" is executed
    Then Result should contain 4 rows with expected boundary values

  @python_e2e @jdbc_e2e
  Scenario: should handle high precision boundaries from table for number and synonyms
    Given Snowflake client is logged in
    And Table with columns (<type>(38,0), <type>(38,37)) exists
    And Row (99999999999999999999999999999999999999, 1.2345678901234567890123456789012345678) is inserted
    And Row (-99999999999999999999999999999999999999, -1.2345678901234567890123456789012345678) is inserted
    And Row (12345678901234567890123456789012345678, 0.0000000000000000000000000000000000001) is inserted
    When Query "SELECT * FROM <table>" is executed
    Then Result should contain 3 rows with expected high precision boundary values

  @python_e2e @jdbc_e2e
  Scenario: should handle NULL values from table with multiple scales for number and synonyms
    Given Snowflake client is logged in
    And Table with columns (<type>(10,0), <type>(10,2), <type>(15,3)) exists
    And Row (NULL, NULL, NULL) is inserted
    And Row (123, 123.45, 123.456) is inserted
    And Row (NULL, NULL, NULL) is inserted
    And Row (-456, -67.89, -789.012) is inserted
    When Query "SELECT * FROM <table>" is executed
    Then Result should contain 4 rows with 2 NULL rows and 2 non-NULL rows with expected values

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should download large result set from table for number and synonyms
    Given Snowflake client is logged in
    And Table with columns (<type>(38,0), <type>(20,5)) exists with 30000 sequential rows, from 0 to 29999 in the first column and from 0.12345 to 29999.12345 in the second column
    When Query "SELECT * FROM <table>" is executed
    Then Column 1 should contain sequential integers from 0 to 29999
    And Column 2 should contain sequential decimals starting from 0.12345

  # =========================================================================== #
  #                            Parameter binding                                #
  # =========================================================================== #

  @python_e2e
  Scenario: should select number using parameter binding for number and synonyms
    Given Snowflake client is logged in
    When Query "SELECT ?::<type>(10,0), ?::<type>(10,0), ?::<type>(10,2), ?::<type>(10,2), ?::<type>(10,0)" is executed with bound values [123, -456, 12.34, -56.78, NULL]
    Then Result should contain [123, -456, 12.34, -56.78, NULL]

  @python_e2e
  Scenario: should select high precision number using parameter binding for number and synonyms
    Given Snowflake client is logged in
    When Query "SELECT ?::<type>(38,0), ?::<type>(38,2)" is executed with bound values [12345678901234567890123456789012345678, 123456789012345678901234567890123456.78]
    Then Result should contain [12345678901234567890123456789012345678, 123456789012345678901234567890123456.78]

  @python_e2e
  Scenario: should insert number using parameter binding for number and synonyms
    Given Snowflake client is logged in
    And Table with columns (<type>(10,0), <type>(10,2)) exists
    When Rows (0, 0.00), (123, 123.45), (-456, -67.89), (999999, 999.99), (NULL, NULL) are inserted using binding
    Then Result should contain 5 rows with expected values

  @python_e2e
  Scenario: should insert high precision number using parameter binding for number and synonyms
    Given Snowflake client is logged in
    And Table with columns (<type>(38,0), <type>(38,2)) exists
    When Rows (12345678901234567890123456789012345678, 123456789012345678901234567890123456.78), (99999999999999999999999999999999999999, 0.01), (-99999999999999999999999999999999999999, -0.01) are inserted using binding
    Then Result should contain 3 rows with expected values keeping the precision

