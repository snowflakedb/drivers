@core
Feature: Sync Query Retry with RequestId
  As a driver user
  I want sync queries to automatically retry with the same requestId on connection failures
  So that query results are not lost due to transient network issues

  @core_int
  Scenario: should include request id in query parameters
    Given a server that captures the request
    When I execute a sync query
    Then the request should include requestId and request_guid parameters

  @core_int
  Scenario: should retry sync query on connection reset
    Given a server that resets on first connection
    When a query is submitted in sync mode
    Then the driver should retry with the same requestId
    And the retry should include retry=true parameter

  @core_int
  Scenario: should use sync mode by default
    Given a server that captures the request
    When I execute a query with Blocking mode
    Then the request body should have asyncExec=false
