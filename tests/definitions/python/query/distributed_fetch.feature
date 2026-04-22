@python
Feature: Distributed fetch (Python)

  @python_e2e
  Scenario: should fetch all rows when batches are pickled and fetched in parallel threads
    Given Snowflake client is logged in
    When Query "SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => 100000)) v" is executed
    And get_result_batches is called
    Then The sum of batch rowcounts should equal 100000
    And The inline batch should have None for compressed_size and uncompressed_size
    And Every remote batch should have positive compressed_size and uncompressed_size
    And Each batch is individually serialized with pickle
    And A thread pool is started with up to 4 workers
    And Each thread deserializes its batch, opens a fresh connection, and iterates rows
    Then The combined row count across all threads should be 100000
    And All fetched ids from 0 to 99999 should be present exactly once
