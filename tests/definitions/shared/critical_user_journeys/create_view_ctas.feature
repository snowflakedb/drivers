@python
Feature: CREATE VIEW / CREATE TABLE AS SELECT

  View creation and CTAS operations.
  Used by Snowpark for create_or_replace_view() and save_as_table(),
  SQLAlchemy for views.

  @python_e2e
  Scenario: should create view and query filtered data
    Given Snowflake client is logged in
    And A source table with 3 rows of test data exists
    When A view is created that filters rows where id > 1
    Then SELECT from the view should return 2 rows

  @python_e2e
  Scenario: should create table as select
    Given Snowflake client is logged in
    And A source table with 3 rows of test data exists
    When CREATE TABLE AS SELECT is executed filtering val > 2.0
    Then The new table should contain the filtered rows
