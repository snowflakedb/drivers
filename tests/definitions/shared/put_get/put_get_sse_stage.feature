@core @python
Feature: PUT/GET on stages with server-side encryption (SNOWFLAKE_SSE)

  @core_e2e @python_e2e
  Scenario: should put file to SSE stage
    Given Stage with server-side encryption (SNOWFLAKE_SSE)
    When File is uploaded using PUT command
    Then File should be uploaded successfully

  @core_e2e @python_e2e
  Scenario: should get file from SSE stage
    Given File is uploaded to stage with server-side encryption (SNOWFLAKE_SSE)
    When File is downloaded using GET command
    Then File should be downloaded
    And Have correct content

  @core_e2e @python_e2e
  Scenario: should put file to SSE stage with DIRECTORY enabled
    Given Stage with server-side encryption and DIRECTORY enabled
    When File is uploaded using PUT command
    Then File should be uploaded successfully
