@core @python @odbc @jdbc
Feature: PUT/GET overwrite

  @core_e2e @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should overwrite file when OVERWRITE is set to true
    Given File is uploaded to stage
    When Updated file is uploaded with OVERWRITE set to true
    Then UPLOADED status is returned
    And File was overwritten

  @core_e2e @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should not overwrite file when OVERWRITE is set to false
    Given File is uploaded to stage
    When Updated file is uploaded with OVERWRITE set to false
    Then SKIPPED status is returned
    And File was not overwritten

  @core_e2e
  Scenario Outline: should skip upload when content matches under overwrite true on gcs <encryption_type>
    Given File is uploaded to a GCS <encryption_type> stage
    When Same file is uploaded again with OVERWRITE set to true
    Then SKIPPED status is returned

    Examples:
      | encryption_type |
      | SSE             |
      | CSE             |