@python @core_not_needed
Feature: BINARY type support
  # Snowflake Binary types: BINARY, VARBINARY
  # Stores binary data (byte sequences) in hexadecimal format
  # Storage format: Internally stored as hexadecimal, returned in HEX or BASE64 based on BINARY_OUTPUT_FORMAT
  # Reference: https://docs.snowflake.com/en/sql-reference/data-types-text#binary

  # =========================================================================== #
  #                               Type casting                                  #
  # =========================================================================== #

  @python_e2e
  Scenario: should cast binary values to appropriate type
    # Python: Values should be cast to 'bytearray' type
    Given Snowflake client is logged in
    When Query "SELECT TO_BINARY('48656C6C6F', 'HEX')::BINARY, TO_BINARY('V29ybGQ=', 'BASE64')::BINARY" is executed
    Then All values should be returned as appropriate binary type
    And the result should contain binary values:
      | col1         | col2         |
      | 0x48656C6C6F | 0x576F726C64 |

  # =========================================================================== #
  #                     SELECT with literals (no tables)                        #
  # =========================================================================== #

  @python_e2e
  Scenario: should select binary literals
    Given Snowflake client is logged in
    When Queries selecting binary literals are executed:
    # SELECT X'48656C6C6F'::{type}
    # SELECT TO_BINARY('48656C6C6F', 'HEX')::{type}
    # SELECT TO_BINARY('ASNFZ4mrze8=', 'BASE64')::{type}
    Then the results should contain expected binary values
      | bin1         | bin2         | bin3               |
      | 0x48656C6C6F | 0x48656C6C6F | 0x0123456789ABCDEF |


  @python_e2e
  Scenario: should handle binary corner case values from literals
    # Corner cases for binary:
    #   - Empty binary: X'' (0 bytes)
    #   - Single byte min: X'00' (null byte)
    #   - Single byte max: X'FF' (max single byte value)
    #   - All zeros: X'0000000000' (5 null bytes)
    #   - All ones: X'FFFFFFFFFF' (5 bytes of 0xFF)
    #   - Embedded nulls: X'48006500' (bytes with embedded 0x00)
    Given Snowflake client is logged in
    When Query selecting corner case binary literals is executed
    Then the result should contain expected corner case binary values

  @python_e2e
  Scenario: should handle NULL binary values from literals
    Given Snowflake client is logged in
    When Query "SELECT NULL::{type}, X'ABCD', NULL::{type}" is executed
    Then Result should contain [NULL, 0xABCD, NULL]

  # =========================================================================== #
  #                     SELECT FROM TABLE (Happy path, Corner cases)            #
  # =========================================================================== #

  @python_e2e
  Scenario: should select binary values from table
    Given Snowflake client is logged in
    And A temporary table with BINARY column is created
    And The table is populated with binary values [X'48656C6C6F', X'576F726C64', X'0123456789ABCDEF']
    When Query "SELECT * FROM {table} ORDER BY col" is executed
    Then the result should contain binary values in order:
      | col                |
      | 0x0123456789ABCDEF |
      | 0x48656C6C6F       |
      | 0x576F726C64       |

  @python_e2e
  Scenario: should select corner case binary values from table
    Given Snowflake client is logged in
    And A temporary table with BINARY column is created
    And The table is populated with corner case binary values
    # Corner cases:
    #   - Empty binary: X'' (0 bytes)
    #   - Single null byte: X'00'
    #   - Single max byte: X'FF'
    #   - Multiple null bytes: X'000000'
    #   - Embedded nulls: X'48006500'
    When Query "SELECT * FROM {table} ORDER BY 1" is executed
    Then the result should contain the inserted corner case binary values

  @python_e2e
  Scenario: should select NULL binary values from table
    Given Snowflake client is logged in
    And A temporary table with BINARY column is created
    And The table is populated with NULL and non-NULL binary values [NULL, X'ABCD', NULL]
    When Query "SELECT * FROM {table}" is executed
    Then there are 3 rows returned
    And 2 rows should contain NULL values
    And 1 row should contain 0xABCD

  @python_e2e
  Scenario: should select binary with specified length from table
    # Tests BINARY(n) with specific length constraints
    Given Snowflake client is logged in
    And Table with columns (bin5 BINARY(5), bin10 BINARY(10), bin_default BINARY) exists
    And Row (X'0102030405', X'01020304050607080910', X'48656C6C6F') is inserted
    When Query "SELECT * FROM {table}" is executed
    Then Result should contain binary values with correct lengths

  # =========================================================================== #
  #                            Parameter binding                                #
  # =========================================================================== #

  @python_e2e
  Scenario: should select binary literals using parameter binding
    # SELECT binding test: Uses SELECT ?::BINARY to bind binary values
    Given Snowflake client is logged in
    When Query "SELECT ?::BINARY, ?::BINARY, ?::BINARY" is executed with bound binary values [0x48656C6C6F, 0x576F726C64, 0x0123456789ABCDEF]
    Then the result should contain:
      | col1           | col2           | col3               |
      | 0x48656C6C6F   | 0x576F726C64   | 0x0123456789ABCDEF |

  @python_e2e @odbc_e2e
  Scenario: should insert binary using parameter binding
    Given Snowflake client is logged in
    And Table with BINARY column exists
    When Binary values [0x48656C6C6F, 0x576F726C64, 0x00, 0xFF, 0x] are inserted using binding
    And Query "SELECT * FROM {table}" is executed
    Then Result should contain binary values [0x48656C6C6F, 0x576F726C64, 0x00, 0xFF, 0x]

  @python_e2e @odbc_e2e
  Scenario: should bind corner case binary values
    Given Snowflake client is logged in
    When Query "SELECT ?::BINARY" is executed with each corner case binary value bound
    # Corner cases:
    #   - Empty binary: b'' (0 bytes)
    #   - Single null byte: b'\x00'
    #   - Single max byte: b'\xff'
    #   - Embedded nulls: b'\x48\x00\x65\x00'
    #   - NULL value
    Then the result should match the bound corner case value

  # =========================================================================== #
  #                       Multiple chunks downloading                           #
  # =========================================================================== #

  @python_e2e
  Scenario: should download binary data in multiple chunks using GENERATOR
    # ~30000 values ensures data is downloaded in at least two chunks
    Given Snowflake client is logged in
    When Query "SELECT seq8() AS id, TO_BINARY(LPAD(TO_VARCHAR(seq8()), 10, '0'), 'UTF-8') AS bin_val FROM TABLE(GENERATOR(ROWCOUNT => 30000)) v ORDER BY id" is executed
    Then there are 30000 rows returned
    And all returned binary values should match the generated values in order

  @python_e2e
  Scenario: should download binary data in multiple chunks from table
    Given Snowflake client is logged in
    And Table with (bin_data BINARY) exists with 30000 sequential binary values
    When Query "SELECT * FROM {table} ORDER BY bin_data" is executed
    Then there are 30000 rows returned
    And all returned binary values should match the inserted values in order
