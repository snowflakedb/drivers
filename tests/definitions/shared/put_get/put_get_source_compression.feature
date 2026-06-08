@core @python @odbc
Feature: PUT/GET source compression

  @core_e2e @python_e2e @odbc_e2e
  Scenario: should auto-detect standard compression types when SOURCE_COMPRESSION set to AUTO_DETECT
    Given Snowflake client is logged in
    And File with standard type (GZIP, BZIP2, BROTLI, ZSTD, DEFLATE)
    When File is uploaded with SOURCE_COMPRESSION set to AUTO_DETECT
    Then Target compression has correct type and all PUT results are correct

  @core_e2e @python_e2e @odbc_e2e
  Scenario: should upload compressed files with SOURCE_COMPRESSION set to explicit types
    Given Snowflake client is logged in
    And File with standard type (GZIP, BZIP2, BROTLI, ZSTD, DEFLATE, RAW_DEFLATE)
    When File is uploaded with SOURCE_COMPRESSION set to explicit type
    Then Target compression has correct type and all PUT results are correct

  @core_e2e @python_e2e @odbc_e2e
  Scenario: should not compress file when SOURCE_COMPRESSION set to AUTO_DETECT and AUTO_COMPRESS set to FALSE
    Given Snowflake client is logged in
    And Uncompressed file
    When File is uploaded with SOURCE_COMPRESSION set to AUTO_DETECT and AUTO_COMPRESS set to FALSE
    Then File is not compressed

  @core_e2e @python_e2e @odbc_e2e
  Scenario: should not compress file when SOURCE_COMPRESSION set to NONE and AUTO_COMPRESS set to FALSE
    Given Snowflake client is logged in
    And Uncompressed file
    When File is uploaded with SOURCE_COMPRESSION set to NONE and AUTO_COMPRESS set to FALSE
    Then File is not compressed

  @core_e2e @python_e2e @odbc_e2e
  Scenario: should compress uncompressed file when SOURCE_COMPRESSION set to AUTO_DETECT and AUTO_COMPRESS set to TRUE
    Given Snowflake client is logged in
    And Uncompressed file
    When File is uploaded with SOURCE_COMPRESSION set to AUTO_DETECT and AUTO_COMPRESS set to TRUE
    Then Target compression has GZIP type and all PUT results are correct

  @core_e2e @python_e2e @odbc_e2e
  Scenario: should compress uncompressed file when SOURCE_COMPRESSION set to NONE and AUTO_COMPRESS set to TRUE
    Given Snowflake client is logged in
    And Uncompressed file
    When File is uploaded with SOURCE_COMPRESSION set to NONE and AUTO_COMPRESS set to TRUE
    Then Target compression has GZIP type and all PUT results are correct

  @core_int @python_int
  Scenario: should return error for unsupported compression type
    Given Snowflake client is logged in
    And File compressed with unsupported format
    When File is uploaded with SOURCE_COMPRESSION set to AUTO_DETECT
    Then Unsupported compression error is thrown

  # ODBC-only: under the Odbc wrapper preset, sf_core restores the legacy
  # libsnowflakeclient behavior of silently treating unsupported
  # compression formats as uncompressed (BehaviorDifferences.yaml BD#6).
  # The original payload is wrapped verbatim inside an outer gzip (because
  # AUTO_COMPRESS=TRUE then runs over the "uncompressed" source) and must
  # roundtrip byte-for-byte through the upload+download cycle.
  @odbc_e2e
  Scenario: should silently upload file with unsupported compression type as uncompressed
    Given Snowflake client is logged in
    And File compressed with unsupported format
    When File is uploaded with SOURCE_COMPRESSION set to AUTO_DETECT
    Then Upload succeeds and the file is treated as uncompressed source
    And Downloaded payload decompresses to the original file bytes

