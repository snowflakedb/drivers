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
  # FILE is currently only implemented for Python. ODBC, JDBC and Node.js are
  # tracked as TODO until FILE support lands in their respective wrappers.

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
