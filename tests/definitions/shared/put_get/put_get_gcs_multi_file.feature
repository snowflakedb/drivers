@core @odbc @dotnet_not_needed
Feature: PUT/GET multi-file GET on GCS (presigned URL pipeline)

  @core_e2e @odbc_e2e @dotnet_not_needed
  Scenario: should get multiple files from stage in single command
    Given Two files are uploaded to stage
    When All files are downloaded from stage using GET command
    Then All files should be downloaded
    And Each file should have correct content
