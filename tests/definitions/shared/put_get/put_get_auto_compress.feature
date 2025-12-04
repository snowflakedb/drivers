@core @python @odbc
Feature: PUT/GET auto-compress

  @core_e2e @python_e2e @odbc_e2e
  Scenario: should compress the file before uploading to stage when AUTO_COMPRESS set to true
    Given Snowflake client is logged in
    When File is uploaded to stage with AUTO_COMPRESS set to true
    Then Only compressed file should be downloaded
    And Have correct content

  @core_e2e @python_e2e @odbc_e2e
  Scenario: should not compress the file before uploading to stage when AUTO_COMPRESS set to false
    Given Snowflake client is logged in
    When File is uploaded to stage with AUTO_COMPRESS set to false
    Then Only uncompressed file should be downloaded
    And Have correct content