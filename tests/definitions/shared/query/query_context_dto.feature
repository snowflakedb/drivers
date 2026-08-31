@core
Feature: Query context DTO cache

  @core_int
  Scenario: should send cached query context in subsequent requests
    Given a wiremock server with login and query response containing queryContext
    When the client executes two queries
    Then the second request contains the cached queryContextDTO entries

  @core_int
  Scenario: should keep cache unchanged when response has no queryContext
    Given a wiremock server with response that has no queryContext field
    When the client executes three queries
    Then all three queries complete without error and the third request still contains the cached entries from the seed response

  @core_int
  Scenario: should clear cache when response has null entries
    Given a wiremock server with response that has null queryContext entries
    When the client executes three queries
    Then all three queries complete without error and the third request has no cached entries

  @core_int
  Scenario: should merge entries when response IDs overlap
    Given a wiremock server with seed and overlap merge responses
    When the client executes three queries
    Then the third request contains merged queryContextDTO entries

  @core_int
  Scenario: should evict highest priority number when cache exceeds capacity
    Given a wiremock server with 4 entries and QUERY_CONTEXT_CACHE_SIZE 3
    When the client executes two queries
    Then the second request has 3 entries with highest priority number evicted

  @core_int
  Scenario: should respect cache size parameter
    Given a wiremock server with 5 entries and QUERY_CONTEXT_CACHE_SIZE 3
    When the client executes two queries
    Then the second request has exactly 3 entries

  @core_int
  Scenario: should update cache on failed query response
    Given a wiremock server that returns an error response with queryContext
    When the client executes a failing query followed by a successful one
    Then the second request carries the context from the error response

  @core_int
  Scenario: should allow duplicate priorities to coexist in cache
    Given a wiremock server with 3 entries sharing the same priority
    When the client executes two queries
    Then the second request contains all 3 entries with the same priority

  @core_int
  Scenario: should evict highest priority number among duplicate priorities
    Given a wiremock server with 4 entries at priority 5 and QUERY_CONTEXT_CACHE_SIZE 3
    When the client executes two queries
    Then the second request has 3 entries and the entry with the lowest timestamp is evicted

  @core_int
  Scenario: should insert new id at occupied priority and evict by capacity
    Given a wiremock server with seed entries and a merge response adding a new id at an existing priority
    When the client executes three queries
    Then the third request contains the new entry and evicts the lowest-importance entry

  @core_int
  Scenario: should re-index entry when priority changes with same timestamp
    Given a wiremock server with seed entry at priority 10 and a merge response changing priority to 5 with same timestamp
    When the client executes three queries
    Then the third request contains the entry with updated priority 5