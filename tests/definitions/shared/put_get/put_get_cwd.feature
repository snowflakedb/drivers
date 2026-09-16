@core @nodejs
Feature: PUT GET cwd

  @core_e2e @nodejs_e2e
  Scenario: should ignore cwd when the PUT source path is absolute
    Given A source file exists at an absolute path
    And cwd points at a different empty directory
    When PUT is executed with that absolute file URI and cwd
    Then The file is uploaded from the absolute path

  @core_e2e @nodejs_e2e
  Scenario: should resolve a relative PUT source path against a relative cwd
    Given A source file exists under a directory relative to the process working directory
    When PUT is executed with a relative file URI and that relative cwd
    Then The file is uploaded from the joined path

  @core_e2e @nodejs_e2e
  Scenario: should resolve a relative PUT source path against an absolute cwd
    Given A source file exists in a temporary directory
    When PUT is executed with a relative file URI and that directory as cwd
    Then The file is uploaded from the joined path

  @core_e2e @nodejs_e2e
  Scenario: should ignore cwd when the GET destination path is absolute
    Given A file exists on a stage
    And cwd points at a different empty directory
    When GET is executed with an absolute destination URI and cwd
    Then The file is downloaded to the absolute path

  @core_e2e @nodejs_e2e
  Scenario: should resolve a relative GET destination against a relative cwd
    Given A file exists on a stage
    And a destination directory exists relative to the process working directory
    When GET is executed with a relative destination URI and that relative cwd
    Then The file is downloaded to the joined path

  @core_e2e @nodejs_e2e
  Scenario: should resolve a relative GET destination against an absolute cwd
    Given A file exists on a stage
    And a destination directory exists in a temporary directory
    When GET is executed with a relative destination URI and that directory as cwd
    Then The file is downloaded to the joined path
