@core
Feature: Session Logout - Core HTTP Layer Integration

  # Low-level HTTP protocol validation and core integration tests.
  # These test UD Core implementation details not exposed to wrappers.

  # ===========================================================================
  #                      HTTP Request Construction
  # ===========================================================================

  Scenario: should construct logout request with correct HTTP method URL headers and body
    Given Mock HTTP server is configured to capture requests
    And UD Core client is logged in with session token
    When Logout is initiated
    Then HTTP method is POST
    And Request URL path is /session
    And Query parameter delete is set to true
    And Query parameter requestId is present and static across attempts
    And Query parameter request_guid is present and unique per attempt
    And Authorization header is present with format "Snowflake Token={session_token}"
    And Content-Type header is application/json
    And Accept header is application/snowflake
    And User-Agent header contains UD version and Rust version
    And Request body is exactly empty JSON object {}

  Scenario: should apply retry policy to logout HTTP request
    Given Mock HTTP server returns 503 error on first attempt
    And Mock HTTP server returns 200 on second attempt
    And Retry policy allows 2 attempts
    When Logout is initiated
    Then First request receives 503 response
    And Retry policy is consulted
    And Second request is made after backoff delay
    And Logout succeeds

  Scenario: should handle HTTP connection reset during logout
    Given Mock HTTP server resets connection on first attempt
    And Mock HTTP server succeeds on second attempt
    When Logout is initiated
    Then Connection reset is detected
    And Request is retried according to retry policy
    And Logout succeeds on retry

  Scenario: should record connection close decision metrics before logout
    Given Telemetry client is configured
    And UD Core client is logged in
    When Connection close is initiated
    Then Pre-logout metrics are recorded in telemetry batch
    And Metrics include whether auto-detection was performed
    And Metrics include whether async queries were detected
    And Metrics include whether logout will be sent or skipped
    And Metrics include skip reason if logout is skipped
    And Telemetry batch is flushed before logout is sent
    And Logout proceeds after telemetry flush completes
