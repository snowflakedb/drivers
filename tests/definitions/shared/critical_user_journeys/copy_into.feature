@python
Feature: COPY INTO (Bulk Loading)

  Bulk data loading via PUT + COPY INTO.
  Used by SQLAlchemy, snowflake-cli, Snowpark, and Snowfort.

  @python_e2e
  Scenario: should bulk load CSV data via PUT and COPY INTO
    Given Snowflake client is logged in
    And A temporary stage "copy_test_stage" exists
    And A temporary table "copy_test" with columns (id INT, name VARCHAR, val FLOAT) exists
    And A local CSV file with 3 rows of test data exists
    When The CSV file is PUT to the stage
    Then LS @copy_test_stage should show 1 file
    When COPY INTO copy_test FROM @copy_test_stage is executed
    Then SELECT * FROM copy_test should return 3 rows with correct values
