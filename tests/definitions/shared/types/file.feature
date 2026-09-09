@python @core_not_needed
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

  @python_e2e
  Scenario: should cast FILE values to appropriate type
    # Python: a FILE column is returned as str, the raw undecoded JSON document
    Given Snowflake client is logged in
    When A FILE column is populated via TO_FILE and queried
    Then the FILE column should be returned as appropriate type with the expected JSON document

  # =========================================================================== #
  #                               Column metadata                               #
  # =========================================================================== #

  @python_e2e
  Scenario: should report a FILE column with a dedicated type code
    # cursor.description[i].type_code reports FILE, matching the code legacy
    # snowflake-connector-python assigns, instead of falling back to TEXT
    Given Snowflake client is logged in
    When A FILE column is populated via TO_FILE and queried
    Then the column should report type code FILE

  # =========================================================================== #
  #                     SELECT with literals (no tables)                        #
  # =========================================================================== #

  @python_e2e
  Scenario: should select a FILE value built by TO_FILE without a table
    Given Snowflake client is logged in
    When Query "SELECT TO_FILE(PARSE_JSON('{"RELATIVE_PATH": "some_new_file.jpeg", ...}'))" is executed
    Then the result should contain the expected FILE JSON document

  @python_e2e
  Scenario: should handle NULL FILE values from literals
    # TO_FILE(NULL) yields a NULL FILE; NULL::FILE is rejected by the server,
    # so the NULL literal is produced through TO_FILE.
    Given Snowflake client is logged in
    When Query "SELECT TO_FILE(PARSE_JSON('...')), TO_FILE(NULL)" is executed
    Then the result should contain the expected FILE JSON document and NULL

  # =========================================================================== #
  #                           Table operations                                  #
  # =========================================================================== #

  @python_e2e
  Scenario: should select FILE values from table
    Given Snowflake client is logged in
    And A temporary table with an ID and a FILE column is created
    And The table is populated with two different FILE values
    When Query "SELECT * FROM {table} ORDER BY ID" is executed
    Then the result should contain the inserted FILE JSON documents in order

  @python_e2e
  Scenario: should handle NULL FILE values from table
    Given Snowflake client is logged in
    And A temporary table with an ID and a FILE column is created
    And The table is populated with a FILE value, a NULL and another FILE value
    When Query "SELECT * FROM {table} ORDER BY ID" is executed
    Then the result should contain the inserted FILE JSON documents and NULL in order

  # =========================================================================== #
  #                       Multiple chunks downloading                           #
  # =========================================================================== #

  @python_e2e
  Scenario: should download FILE data in multiple chunks
    Given Snowflake client is logged in
    When Query generating 20000 FILE values is executed
    Then All 20000 rows should be fetched with the expected FILE JSON documents
