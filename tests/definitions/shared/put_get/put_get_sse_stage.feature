@core @python
Feature: PUT/GET on stages with server-side encryption (SNOWFLAKE_SSE)

  @core_e2e @python_e2e
  Scenario: should put and get file on SSE stage
    Given Stage with server-side encryption (SNOWFLAKE_SSE)
    When File is uploaded using PUT command
    Then File should be uploaded successfully
    When File is downloaded using GET command
    Then File should be downloaded
    And Have correct content

  @core_e2e @python_e2e
  Scenario: should put and get file on SSE stage with DIRECTORY enabled
    Given Stage with server-side encryption and DIRECTORY enabled
    When File is uploaded using PUT command
    Then File should be uploaded successfully
    When File is downloaded using GET command
    Then File should be downloaded
    And Have correct content
