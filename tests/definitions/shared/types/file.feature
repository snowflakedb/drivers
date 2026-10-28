@python @core_not_needed @odbc
Feature: FILE type support
  # Snowflake FILE type represents a reference to a staged file (e.g. produced by
  # TO_FILE) as a JSON document describing it (RELATIVE_PATH, STAGE,
  # STAGE_FILE_URL, SIZE, ETAG, CONTENT_TYPE, LAST_MODIFIED, ...).
  # Reference: https://docs.snowflake.com/en/sql-reference/data-types-unstructured
  #
  # No driver decodes the JSON document: a FILE column is read as the raw,
  # undecoded string the server sends, the same representation used for VARIANT.
  #
  # A FILE value has no client-side representation, so it cannot be bound
  # directly: a parameter bound into a FILE column is resolved as a stage path
  # and fails unless that staged file exists.
  #
  # TO_FILE rejects a metadata document that omits any of RELATIVE_PATH, STAGE,
  # STAGE_FILE_URL, SIZE, ETAG, CONTENT_TYPE or LAST_MODIFIED, so every document
  # below carries the full set.

  # =========================================================================== #
  #                               Type casting                                  #
  # =========================================================================== #

  @python_e2e @odbc_e2e
  Scenario: should cast FILE values to appropriate type
    # Python: a FILE column is returned as str, the raw undecoded JSON document
    Given Snowflake client is logged in
    When A FILE column is populated via TO_FILE and queried
    Then the FILE column should be returned as appropriate type with the expected JSON document

  # =========================================================================== #
  #                               Column metadata                               #
  # =========================================================================== #

  # - ODBC SQLDescribeCol reports SQL_VARCHAR. There is no ODBC SQL type for FILE.
  #   SQLColAttribute(SQL_DESC_TYPE_NAME) reports FILE, which distinguishes a FILE
  #   column from VARIANT. SQLDescribeCol stays SQL_VARCHAR when
  #   ODBC_USE_CUSTOM_SQL_DATA_TYPES is enabled.
  # - Python cursor.description[i].type_code reports FILE.
  # - Node returns file as "object"
  @python_e2e @odbc_not_needed @nodejs_e2e_not_needed
  Scenario: should report a FILE column with a dedicated type code
    # cursor.description[i].type_code reports FILE, matching the code legacy
    # snowflake-connector-python assigns, instead of falling back to TEXT
    Given Snowflake client is logged in
    When A FILE column is populated via TO_FILE and queried
    Then the column should report type code FILE

  @odbc_e2e @python_not_needed
  Scenario: should report FILE as the column type name
    Given Snowflake client is logged in
    When A FILE column is populated via TO_FILE and queried
    Then SQLDescribeCol reports SQL_VARCHAR and SQL_DESC_TYPE_NAME reports FILE

  @odbc_e2e @python_not_needed
  Scenario Outline: should keep SQL_VARCHAR for FILE when ODBC_USE_CUSTOM_SQL_DATA_TYPES is <flag_state>
    Given Snowflake client is logged in
    And Session parameter ODBC_USE_CUSTOM_SQL_DATA_TYPES is <flag_state>
    When A FILE column is populated via TO_FILE and queried
    Then SQLDescribeCol reports SQL_VARCHAR and SQL_DESC_TYPE_NAME reports FILE

    Examples:
      | flag_state |
      | FALSE      |
      | TRUE       |

  # =========================================================================== #
  #                          ODBC value retrieval                               #
  # =========================================================================== #

  @odbc_e2e @python_not_needed
  Scenario: should retrieve FILE data as SQL_C_WCHAR
    Given Snowflake client is logged in
    When A FILE column is populated via TO_FILE and queried
    Then the FILE column should be returned as SQL_C_WCHAR with the expected JSON document

  @odbc_e2e @python_not_needed
  Scenario: should retrieve FILE data as SQL_C_BINARY
    Given Snowflake client is logged in
    When A FILE column is populated via TO_FILE and queried
    Then the FILE column should be returned as SQL_C_BINARY with the expected JSON document

  @odbc_e2e @python_not_needed
  Scenario: should return SQL_NULL_DATA for NULL FILE as SQL_C_BINARY
    Given Snowflake client is logged in
    When Query "SELECT TO_FILE(NULL)" is executed
    Then the indicator should be SQL_NULL_DATA

  @odbc_e2e @python_not_needed
  Scenario: should truncate FILE data when the character buffer is too short
    Given Snowflake client is logged in
    When A FILE column is populated via TO_FILE and queried
    Then SQLGetData should return SQL_SUCCESS_WITH_INFO with SQLSTATE 01004

  @odbc_e2e @python_not_needed
  Scenario: should truncate FILE data as SQL_C_WCHAR when the buffer is too short
    Given Snowflake client is logged in
    When A FILE column is populated via TO_FILE and queried
    Then SQLGetData should return SQL_SUCCESS_WITH_INFO with SQLSTATE 01004

  # =========================================================================== #
  #                     SELECT with literals (no tables)                        #
  # =========================================================================== #

  @python_e2e @odbc_e2e
  Scenario: should select a FILE value built by TO_FILE without a table
    Given Snowflake client is logged in
    When Query "SELECT TO_FILE(PARSE_JSON('{"RELATIVE_PATH": "some_new_file.jpeg", ...}'))" is executed
    Then the result should contain the expected FILE JSON document

  @python_e2e @odbc_e2e
  Scenario: should handle NULL FILE values from literals
    # TO_FILE(NULL) yields a NULL FILE; NULL::FILE is rejected by the server,
    # so the NULL literal is produced through TO_FILE.
    Given Snowflake client is logged in
    When Query "SELECT TO_FILE(PARSE_JSON('...')), TO_FILE(NULL)" is executed
    Then the result should contain the expected FILE JSON document and NULL

  # =========================================================================== #
  #                           Table operations                                  #
  # =========================================================================== #

  @python_e2e @odbc_e2e
  Scenario: should select FILE values from table
    Given Snowflake client is logged in
    And A temporary table with an ID and a FILE column is created
    And The table is populated with two different FILE values
    When Query "SELECT * FROM {table} ORDER BY ID" is executed
    Then the result should contain the inserted FILE JSON documents in order

  @python_e2e @odbc_e2e
  Scenario: should handle NULL FILE values from table
    Given Snowflake client is logged in
    And A temporary table with an ID and a FILE column is created
    And The table is populated with a FILE value, a NULL and another FILE value
    When Query "SELECT * FROM {table} ORDER BY ID" is executed
    Then the result should contain the inserted FILE JSON documents and NULL in order

  # =========================================================================== #
  #                       Multiple chunks downloading                           #
  # =========================================================================== #

  @python_e2e @odbc_e2e
  Scenario: should download FILE data in multiple chunks
    Given Snowflake client is logged in
    When Query generating 20000 FILE values is executed
    Then All 20000 rows should be fetched with the expected FILE JSON documents
