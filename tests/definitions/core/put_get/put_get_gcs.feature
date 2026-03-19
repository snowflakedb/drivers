@core
Feature: PUT/GET operations on GCS

  Tests are skipped on non-GCP accounts.

  @core_e2e
  Scenario: GCS should put and get file
    Given GCP-backed Snowflake account
    When File is uploaded and downloaded via GCS stage
    Then Round-trip content should match

  @core_e2e
  Scenario: GCS should put overwrite and get
    Given GCP-backed Snowflake account
    When File is uploaded twice with overwrite
    Then Second upload should succeed with UPLOADED status

  @core_e2e
  Scenario: GCS should skip existing file without overwrite
    Given GCP-backed Snowflake account
    When File is uploaded twice without overwrite
    Then Second upload should return SKIPPED status

  @core_e2e
  Scenario: GCS should put and get multiple files
    Given GCP-backed Snowflake account
    When Multiple files are uploaded and downloaded
    Then Each file should have correct content

  @core_e2e
  Scenario: GCS should return correct PUT rowset
    Given GCP-backed Snowflake account
    When File is uploaded to GCS stage
    Then PUT rowset should have correct metadata

  @core_e2e
  Scenario: GCS should return correct GET rowset
    Given GCP-backed Snowflake account
    When File is downloaded from GCS stage
    Then GET rowset should have correct metadata
