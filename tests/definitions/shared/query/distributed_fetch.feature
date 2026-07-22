@core @python @jdbc
Feature: Distributed fetch

  # A large result set is split into independently serializable partitions. Each partition can be
  # serialized on its own, handed to another worker, deserialized there, and its rows fetched
  # without sharing the original live session.

  @core_e2e
  Scenario: should return inline chunk for simple distributed fetch query
    Given Snowflake client is logged in
    When Query "SELECT 42 AS answer, 'hello' AS greeting" is executed
    Then result chunks should contain at least one inline chunk
    And fetching the inline chunk should return 1 row with 2 columns

  @core_e2e
  Scenario: should produce multiple chunks for large distributed fetch result
    Given Snowflake client is logged in
    When Large query generating 500000 rows is executed
    Then result chunks should contain at least 2 chunks
    And result chunks should contain at least one remote chunk
    And fetching all chunks in one request should return 500000 total rows

  @python_e2e @jdbc_e2e
  Scenario: should fetch all rows when partitions fetched in parallel threads
    Given Snowflake client is logged in
    And Query "SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => 100000)) v" is executed
    And the result set is split into independently serializable partitions
    Then there should be at least two partitions
    When each partition is serialized and fetched on its own worker thread without a live session
    Then the combined row count across all threads should be 100000
    And all ids from 0 to 99999 should be present exactly once

  @python_e2e @jdbc_e2e
  Scenario: should preserve row count and data sizes across partition split
    Given Snowflake client is logged in
    When Query "SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => 100000)) v" is executed
    And the result set is split into independently serializable partitions
    Then the sum of the partition row counts should be 100000
    And the aggregate compressed and uncompressed data sizes should be preserved across the split

  @jdbc_e2e
  Scenario: should round trip result set through serializable repeatedly
    Given Snowflake client is logged in
    When Query "SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => 100000)) v" is executed
    And the result set is round tripped through a serializable and back to a result set
    And the rehydrated result set is round tripped through a serializable a second time
    Then the twice round tripped result set should expose all 100000 rows
    And all ids from 0 to 99999 should be present exactly once

  @python_e2e
  Scenario: should fetch all rows when batches are pickled and reconnected in parallel threads
    Given Snowflake client is logged in
    And Query "SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => 100000)) v" is executed
    And the result set is split into independently serializable partitions
    When each partition is serialized and fetched on its own worker thread after opening a fresh session
    Then the combined row count across all threads should be 100000
    And all ids from 0 to 99999 should be present exactly once

  @python_e2e @jdbc_e2e
  Scenario: should preserve session timezone for timestamp ltz fetched from serializable without a live session
    Given Snowflake client is logged in with a non-default session timezone
    When a query returning TIMESTAMP_LTZ values is executed
    And the result set is split into independently serializable partitions
    And each partition is serialized and fetched without a live session
    Then the fetched timestamp values should match those rendered by the originating session
