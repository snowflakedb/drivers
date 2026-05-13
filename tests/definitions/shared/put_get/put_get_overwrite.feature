@core @python @odbc
Feature: PUT/GET overwrite

  @core_e2e @python_e2e @odbc_e2e
  Scenario: should overwrite file when OVERWRITE is set to true
    Given File is uploaded to stage
    When Updated file is uploaded with OVERWRITE set to true
    Then UPLOADED status is returned
    And File was overwritten

  @core_e2e @python_e2e @odbc_e2e
  Scenario: should not overwrite file when OVERWRITE is set to false
    Given File is uploaded to stage
    When Updated file is uploaded with OVERWRITE set to false
    Then SKIPPED status is returned
    And File was not overwritten

  # ODBC (legacy libsnowflakeclient) never implemented digest-match skipping,
  # so the `odbc` preset keeps the old unconditional re-upload behavior.
  # Python wrapper test is TODO — the Rust core exposes the behavior via
  # `WrapperPresets::python()`; a Python-level E2E mirroring the connector's
  # `test_put_overwrite_skip_on_content_match` should be added in a follow-up.
  @core_e2e @odbc_not_needed
  Scenario: should skip upload when OVERWRITE is true and remote digest matches
    Given File is uploaded to stage with OVERWRITE set to true
    When Same file is uploaded again with OVERWRITE set to true
    Then SKIPPED status is returned
    And File on stage is unchanged