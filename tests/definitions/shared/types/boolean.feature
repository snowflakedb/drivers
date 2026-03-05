@python @jdbc @odbc
Feature: BOOLEAN type support

  # =========================================================================== #
  #                               Type casting                                  #
  # =========================================================================== #

  @python_e2e @jdbc_e2e @odbc_e2e
  Scenario: should cast boolean values to appropriate type
    # Python: Values should be cast to 'bool' type
    # ODBC: Default type is SQL_C_BIT (0 or 1)
    Given Snowflake client is logged in
    When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN, TRUE::BOOLEAN" is executed
    Then All values should be returned as appropriate type
    And Values should match [TRUE, FALSE, TRUE]

  # =========================================================================== #
  #                     SELECT with literals (no tables)                        #
  # =========================================================================== #

  @python_e2e @jdbc_e2e @odbc_e2e
  Scenario: should select boolean literals
    Given Snowflake client is logged in
    When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN" is executed
    Then Result should contain [TRUE, FALSE]

  @python_e2e @jdbc_e2e @odbc_e2e
  Scenario: should handle NULL values from literals
    Given Snowflake client is logged in
    When Query "SELECT FALSE::BOOLEAN, NULL::BOOLEAN, TRUE::BOOLEAN, NULL::BOOLEAN" is executed
    Then Result should contain [FALSE, NULL, TRUE, NULL]

  @python_e2e @jdbc_e2e @odbc_e2e
  Scenario: should download large result set with multiple chunks from GENERATOR
    Given Snowflake client is logged in
    When Query "SELECT (id % 2 = 0)::BOOLEAN FROM <generator>" is executed
    Then Result should contain 500000 TRUE and 500000 FALSE values

  # =========================================================================== #
  #                             Table operations                                #
  # =========================================================================== #

  @python_e2e @jdbc_e2e @odbc_e2e
  Scenario: should select boolean values from table
    Given Snowflake client is logged in
    And Table with columns (BOOLEAN, BOOLEAN, BOOLEAN) exists
    And Row (TRUE, FALSE, TRUE) is inserted
    When Query "SELECT * FROM <table>" is executed
    Then Result should contain [TRUE, FALSE, TRUE]

  @python_e2e @jdbc_e2e @odbc_e2e
  Scenario: should handle NULL values from table
    Given Snowflake client is logged in
    And Table with BOOLEAN column exists
    And Rows [NULL, TRUE, FALSE] are inserted
    When Query "SELECT * FROM <table>" is executed
    Then Result should contain [NULL, TRUE, FALSE] in any order

  @python_e2e @jdbc_e2e @odbc_e2e
  Scenario: should download large result set with multiple chunks from table
    Given Snowflake client is logged in
    And Table with BOOLEAN column exists with 500000 TRUE and 500000 FALSE values
    When Query "SELECT col FROM <table>" is executed
    Then Result should contain 500000 TRUE and 500000 FALSE values

  # =========================================================================== #
  #                            Parameter binding                                #
  # =========================================================================== #

  @python_e2e @jdbc_e2e @odbc_e2e
  Scenario: should select boolean using parameter binding
    Given Snowflake client is logged in
    When Query "SELECT ?::BOOLEAN, ?::BOOLEAN, ?::BOOLEAN" is executed with bound boolean values [TRUE, FALSE, TRUE]
    Then Result should contain [TRUE, FALSE, TRUE]
    When Query "SELECT ?::BOOLEAN" is executed with bound NULL value
    Then Result should contain [NULL]

  @python_e2e @odbc_e2e
  Scenario: should insert boolean using parameter binding
    Given Snowflake client is logged in
    And Table with BOOLEAN column exists
    When Boolean values [TRUE, FALSE, NULL] are bulk-inserted using multirow binding
    Then SELECT should return the same values in any order
