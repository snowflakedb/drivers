//! Integration tests for session logout functionality.

#[test]
#[ignore = "TODO: SNOW-2872349"]
fn should_return_true_when_first_running_async_query_is_detected_without_checking_remaining_queries(
) {
    //Given Async query registry contains multiple queries
    //And First query in registry is running
    //When Auto-detection checks for running queries
    //Then Detection returns true immediately
    //And Remaining queries are not checked
    todo!()
}

#[test]
#[ignore = "TODO: SNOW-2872349"]
fn should_construct_logout_request_with_correct_http_method_url_headers_and_body() {
    //Given Mock HTTP server is configured to capture requests
    //And UD Core client is logged in with session token
    //When Logout is initiated
    //Then HTTP method is POST
    //And Request URL path is /session
    //And Query parameter delete is set to true
    //And Query parameter requestId is present and static across attempts
    //And Query parameter request_guid is present and unique per attempt
    //And Authorization header is present with format "Snowflake Token={session_token}"
    //And Content-Type header is application/json
    //And Accept header is application/snowflake
    //And User-Agent header contains UD version and Rust version
    //And Request body is exactly empty JSON object {}
    todo!()
}

#[test]
#[ignore = "TODO: SNOW-2872349"]
fn should_apply_retry_policy_to_logout_http_request() {
    //Given Mock HTTP server returns 503 error on first attempt
    //And Mock HTTP server returns 200 on second attempt
    //And Retry policy allows 2 attempts
    //When Logout is initiated
    //Then First request receives 503 response
    //And Retry policy is consulted
    //And Second request is made after backoff delay
    //And Logout succeeds
    todo!()
}

#[test]
#[ignore = "TODO: SNOW-2872349"]
fn should_handle_http_connection_reset_during_logout() {
    //Given Mock HTTP server resets connection on first attempt
    //And Mock HTTP server succeeds on second attempt
    //When Logout is initiated
    //Then Connection reset is detected
    //And Request is retried according to retry policy
    //And Logout succeeds on retry
    todo!()
}

#[test]
#[ignore = "TODO: SNOW-2872349"]
fn should_record_connection_close_decision_metrics_before_logout() {
    //Given Telemetry client is configured
    //And UD Core client is logged in
    //When Connection close is initiated
    //Then Pre-logout metrics are recorded in telemetry batch
    //And Metrics include whether auto-detection was performed
    //And Metrics include whether async queries were detected
    //And Metrics include whether logout will be sent or skipped
    //And Metrics include skip reason if logout is skipped
    //And Telemetry batch is flushed before logout is sent
    //And Logout proceeds after telemetry flush completes
    todo!()
}
