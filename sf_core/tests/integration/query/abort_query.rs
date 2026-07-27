//! `ConnectionAbortQuery` RPC path: the server's raw `success` bool must reach
//! the wrapper as the typed `AbortQueryOutcome`, and a genuine transport
//! failure must surface as an error rather than being collapsed into a
//! declined-abort outcome.

use crate::common::mocks;
use crate::common::snowflake_test_client::SnowflakeTestClient;
use sf_core::protobuf::generated::database_driver_v1::AbortQueryOutcome;
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test(flavor = "multi_thread")]
async fn abort_query_returns_aborted_when_server_acknowledges() {
    let gs_server = MockServer::start().await;
    mocks::auth::mount_jwt_login_success(&gs_server).await;
    mocks::query::mount_abort_query_response(&gs_server, true, /* expected_calls */ 1).await;

    let gs_uri = gs_server.uri();
    tokio::task::spawn_blocking(move || {
        let client = SnowflakeTestClient::connect_integration_test(Some(&gs_uri));
        let outcome = client.abort_query("01abcdef-0000-0000-0000-000000000000");
        assert_eq!(outcome, AbortQueryOutcome::Aborted);
    })
    .await
    .unwrap();
}

/// A completed/not-executing query is a normal server-declined outcome —
/// the server's `success: false` must reach the wrapper as `NotRunning`,
/// not an error.
#[tokio::test(flavor = "multi_thread")]
async fn abort_query_returns_not_running_when_server_declines() {
    let gs_server = MockServer::start().await;
    mocks::auth::mount_jwt_login_success(&gs_server).await;
    mocks::query::mount_abort_query_response(&gs_server, false, /* expected_calls */ 1).await;

    let gs_uri = gs_server.uri();
    tokio::task::spawn_blocking(move || {
        let client = SnowflakeTestClient::connect_integration_test(Some(&gs_uri));
        let outcome = client.abort_query("01abcdef-0000-0000-0000-000000000000");
        assert_eq!(outcome, AbortQueryOutcome::NotRunning);
    })
    .await
    .unwrap();
}

/// Regression for the `Err(_) => false` swallow: a transport failure must
/// surface as an RPC error, not silently masquerade as a declined abort.
#[tokio::test(flavor = "multi_thread")]
async fn abort_query_transport_error_surfaces_as_error_not_silent_false() {
    let gs_server = MockServer::start().await;
    mocks::auth::mount_jwt_login_success(&gs_server).await;
    Mock::given(method("POST"))
        .and(path_regex(r"/queries/.*/abort-request"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .expect(1)
        .mount(&gs_server)
        .await;

    let gs_uri = gs_server.uri();
    tokio::task::spawn_blocking(move || {
        let client = SnowflakeTestClient::connect_integration_test(Some(&gs_uri));
        let result = client.abort_query_no_unwrap("01abcdef-0000-0000-0000-000000000000");
        assert!(
            result.is_err(),
            "expected a genuine RPC error, not a silent success:false; got {result:?}"
        );
    })
    .await
    .unwrap();
}
