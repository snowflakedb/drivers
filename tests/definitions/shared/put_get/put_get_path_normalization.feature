@python @odbc @jdbc
Feature: PUT source path normalization

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should upload file when source path contains dotdot segments
    Given A source file exists in a temporary directory
    When PUT command is executed with a source path containing dotdot segments
    Then File is uploaded successfully with correct target name

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should upload file when source path is relative to working directory
    Given A source file exists in a temporary directory
    When PUT command is executed with a path relative to the process working directory
    Then File is uploaded successfully with correct target name

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should upload file at symlinked source path
    Given A source file and a symlink pointing to it exist in a temporary directory
    When PUT command is executed with the symlink as source path
    Then File is uploaded successfully

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should upload file when source path starts with tilde
    Given A source file exists in a subdirectory under the home directory
    When PUT command is executed with a leading ~ in the source path
    Then File is uploaded successfully
