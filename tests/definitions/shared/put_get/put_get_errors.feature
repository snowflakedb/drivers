@core @python @jdbc
Feature: PUT/GET error handling

  @core_e2e @python_e2e @jdbc_e2e
  Scenario: should return error when putting nonexistent local file
    Given A stage is created
    When PUT is executed with a path to a nonexistent local file
    Then An error is raised indicating the local file does not exist

  @core_e2e @python_e2e
  Scenario: should return error when getting nonexistent file from stage
    Given An empty stage is created
    When GET is executed for a file that does not exist in stage
    Then An error is raised indicating the remote file does not exist

  # JDBC-only: snowflake-jdbc returns an empty GET listing when no staged
  # object matches; other drivers raise (scenario above).
  @jdbc_e2e
  Scenario: should return empty result set when getting nonexistent file from stage
    Given An empty stage is created
    When GET is executed for a file that does not exist in stage
    Then An empty result set is returned
