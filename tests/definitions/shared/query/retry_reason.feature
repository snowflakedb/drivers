@core
Feature: Retry reason context on retried query requests

  When the HTTP retry layer retries a query request, the retried URL includes
  retryCount=N and (when includeRetryReason is enabled) retryReason=<status_code>
  so the server can correlate retries with their causes for observability.

  # ==========================================================================
  # RETRY PARAMS ON SERVER ERRORS (SYNC PATH)
  # ==========================================================================

  @core_int
  Scenario Outline: should include retryReason with HTTP status code on server error retry
    Given a wiremock server that returns <status_code> on the first query then succeeds
    When the client executes a query
    Then the retry request includes retryCount=1
        And the retry request includes retryReason=<status_code>

    Examples:
        | status_code |
        | 503         |
        | 429         |

  # ==========================================================================
  # RETRY PARAMS ON SERVER ERRORS (ASYNC SUBMIT PATH)
  # ==========================================================================

  @core_int
  Scenario: should include retryReason with 503 on async retry
    Given a wiremock server that returns 503 on the first query then succeeds
    When the client executes an async query
    Then the retry request includes retryCount=1
    And the retry request includes retryReason=503

  # ==========================================================================
  # DISABLED FEATURE
  # ==========================================================================

  @core_int
  Scenario: should suppress retryReason when includeRetryReason is disabled
    Given a wiremock server that returns 503 on the first query then succeeds
    And the connection has includeRetryReason set to false
    When the client executes a query
    Then the retry request includes retryCount=1
    But the retry request does not include retryReason

  @core_int
  Scenario: should suppress retryReason on async submit when disabled
    Given a wiremock server that returns 503 on the first query then succeeds
    And the connection has includeRetryReason set to false
    When the client executes an async query
    Then the retry request includes retryCount=1
    But the retry request does not include retryReason

  # ==========================================================================
  # NON-QUERY ENDPOINTS
  # ==========================================================================

  @core_int
  Scenario: should not include retry params on non-query endpoints
    Given a wiremock server that returns 503 on the first login then succeeds
    When the client connects
    Then the login retry request does not include retryCount
    And the login retry request does not include retryReason

  # ==========================================================================
  # RETRY COUNT INCREMENTS AND REASON UPDATES
  # ==========================================================================

  @core_int
  Scenario: should update retryReason on each subsequent retry
    Given a wiremock server that returns 429 then 503 then succeeds on query
    When the client executes a query
    Then the second request includes retryCount=1 and retryReason=429
    And the third request includes retryCount=2 and retryReason=503

  @core_int
  Scenario: should increment retryCount on each retry
    Given a wiremock server that returns 503 twice then succeeds on query
    When the client executes a query
    Then the second request includes retryCount=1
    And the third request includes retryCount=2

  # ==========================================================================
  # FIRST REQUEST HAS NO RETRY PARAMS
  # ==========================================================================

  @core_int
  Scenario: should not include retry params on first request
    Given a wiremock server that succeeds on query
    When the client executes a query
    Then the request does not include retryCount
    And the request does not include retryReason
