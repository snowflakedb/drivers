@core @python @odbc
Feature: PUT/GET basic operations

  @core_e2e @python_e2e @odbc_e2e
  Scenario: should select data from file uploaded to stage
    Given File is uploaded to stage
    When File data is queried using Select command
    Then File data should be correctly returned

  @core_e2e @python_e2e @odbc_e2e
  Scenario: should list file uploaded to stage
    Given File is uploaded to stage
    When Stage content is listed using LS command
    Then File should be listed with correct filename

  @core_e2e @python_e2e @odbc_e2e
  Scenario: should get file uploaded to stage
    Given File is uploaded to stage
    When File is downloaded using GET command
    Then File should be downloaded
    And Have correct content

  @core_e2e @python_e2e @odbc_e2e
  Scenario: should return correct rowset for PUT
    Given Snowflake client is logged in
    When File is uploaded to stage
    Then Rowset for PUT command should be correct

  @core_e2e @python_e2e @odbc_e2e
  Scenario: should return correct rowset for GET
    Given File is uploaded to stage
    When File is downloaded using GET command
    Then Rowset for GET command should be correct

  @core_e2e @python_e2e
  Scenario: should return correct column metadata for PUT
    Given Snowflake client is logged in
    When File is uploaded to stage
    Then Column metadata for PUT command should be correct

  @core_e2e @python_e2e
  Scenario: should return correct column metadata for GET
    Given File is uploaded to stage
    When File is downloaded using GET command
    Then Column metadata for GET command should be correct

  @python_e2e
  Scenario: should upload file to subdirectory in stage
    Given Snowflake client is logged in
    When File is uploaded to a subdirectory in stage
    Then File should be listed under the subdirectory

  @python_e2e
  Scenario: should get file from subdirectory in stage
    Given File is uploaded to a subdirectory in stage
    When All files are downloaded from stage using GET command
    Then File should be downloaded flat into the local directory
    And Have correct content
