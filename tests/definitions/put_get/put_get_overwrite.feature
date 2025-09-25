@core
Feature: PUT/GET overwrite

  @core_e2e
  Scenario: should overwrite file when OVERWRITE is set to true
    Given File is uploaded to stage
    When Updated file is uploaded with OVERWRITE set to true
    Then UPLOADED status is returned
    And File was overwritten

  @core_e2e
  Scenario: should not overwrite file when OVERWRITE is set to false
    Given File is uploaded to stage
    When Updated file is uploaded with OVERWRITE set to false
    Then SKIPPED status is returned
    And File was not overwritten