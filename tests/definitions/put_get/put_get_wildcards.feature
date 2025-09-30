@core
Feature: PUT/GET wildcards
  TODO: scenarios list are not complete, there should be more cases implemented, i.e.:
  - nested directories
  - wildcard matches many files with the same name

  @core_e2e
  Scenario: should upload files that match wildcard question mark pattern
    Given Files matching wildcard pattern
    And Files not matching wildcard pattern
    When Files are uploaded using command with question mark wildcard
    Then Files matching wildcard pattern are uploaded
    And Files not matching wildcard pattern are not uploaded

  @core_e2e
  Scenario: should upload files that match wildcard star pattern
    Given Files matching wildcard pattern
    And Files not matching wildcard pattern
    When Files are uploaded using command with star wildcard
    Then Files matching wildcard pattern are uploaded
    And Files not matching wildcard pattern are not uploaded

  @core_e2e
  Scenario: should download files that are matching wildcard pattern
    Given Files matching wildcard pattern are uploaded
    And Files not matching wildcard pattern are uploaded
    When Files are downloaded using command with wildcard
    Then Files matching wildcard pattern are downloaded
    And Files not matching wildcard pattern are not downloaded