Feature: Put Get basic operations

  Scenario: should select data from file uploaded to stage
    Given File is uploaded to stage
    When File data is queried using Select command
    Then File data should be correctly returned

  Scenario: should list file uploaded to stage
    Given File is uploaded to stage
    When Stage content is listed using LS command
    Then File should be listed with correct filename

  Scenario: should get file uploaded to stage
    Given File is uploaded to stage
    When File is downloaded using GET command
    Then File should be downloaded
    And Have correct content

  Scenario: should return correct rowset for PUT
    Given Snowflake client is logged in
    When File is uploaded to stage
    Then Rowset for PUT command should be correct

  Scenario: should return correct rowset for GET
    Given File is uploaded to stage
    When File is downloaded using GET command
    Then Rowset for GET command should be correct

  Scenario: should return correct column metadata for PUT
    Given Snowflake client is logged in
    When File is uploaded to stage
    Then Column metadata for PUT command should be correct

  Scenario: should return correct column metadata for GET
    Given File is uploaded to stage
    When File is downloaded using GET command
    Then Column metadata for GET command should be correct
