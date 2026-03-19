@python
Feature: MERGE INTO (Upsert)

  MERGE INTO for upsert operations combining UPDATE and INSERT.
  Used by SQLAlchemy's MergeInto construct, Snowfort for replication and OLTP.

  @python_e2e
  Scenario: should merge with update and insert
    Given Snowflake client is logged in
    And A target table with rows (1, 'original_1', 100) and (2, 'original_2', 200) exists
    And A source table with rows (2, 'updated_2', 250) and (3, 'new_3', 300) exists
    When MERGE INTO target USING source is executed with UPDATE on match and INSERT on no match
    Then Merge rowcount should be 2
    And Row id=1 should be untouched as (1, 'original_1', 100)
    And Row id=2 should be updated to (2, 'updated_2', 250)
    And Row id=3 should be inserted as (3, 'new_3', 300)
