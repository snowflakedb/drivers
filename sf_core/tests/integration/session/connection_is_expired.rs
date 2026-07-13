use scopeguard::defer;
use serde_json::json;
use sf_core::protobuf::apis::database_driver_v1::{
    DatabaseDriverClientBlockingExt, database_driver_client,
};
use sf_core::protobuf::generated::database_driver_v1::*;
use wiremock::matchers::{method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::common::mocks::auth::mount_jwt_login_success;
use crate::common::snowflake_test_client::SnowflakeTestClient;

fn setup() -> (
    impl DatabaseDriverClientBlockingExt,
    DatabaseHandle,
    ConnectionHandle,
) {
    let client = database_driver_client();
    let db_handle = client
        .database_new_blocking(DatabaseNewRequest {})
        .unwrap()
        .db_handle
        .unwrap();
    client
        .database_init_blocking(DatabaseInitRequest {
            db_handle: Some(db_handle),
        })
        .unwrap();
    let conn_handle = client
        .connection_new_blocking(ConnectionNewRequest {})
        .unwrap()
        .conn_handle
        .unwrap();
    (client, db_handle, conn_handle)
}

/// A freshly-created connection must not be expired.
#[test]
fn test_connection_is_expired_initially_false() {
    let (client, db_handle, conn_handle) = setup();
    defer! {
        let _ = client.connection_release_blocking(ConnectionReleaseRequest {
            conn_handle: Some(conn_handle),
        });
        let _ = client.database_release_blocking(DatabaseReleaseRequest {
            db_handle: Some(db_handle),
        });
    }

    let is_expired = client
        .connection_is_expired_blocking(ConnectionIsExpiredRequest {
            conn_handle: Some(conn_handle),
        })
        .unwrap()
        .is_expired;
    assert!(!is_expired, "New connection should not be expired");
}

/// Closing a connection must NOT set the expired flag — expired and closed
/// are orthogonal states.
#[test]
fn test_connection_is_expired_not_set_by_close() {
    let (client, db_handle, conn_handle) = setup();
    defer! {
        let _ = client.connection_release_blocking(ConnectionReleaseRequest {
            conn_handle: Some(conn_handle),
        });
        let _ = client.database_release_blocking(DatabaseReleaseRequest {
            db_handle: Some(db_handle),
        });
    }

    client
        .connection_close_blocking(ConnectionCloseRequest {
            conn_handle: Some(conn_handle),
        })
        .unwrap();

    let is_expired = client
        .connection_is_expired_blocking(ConnectionIsExpiredRequest {
            conn_handle: Some(conn_handle),
        })
        .unwrap()
        .is_expired;
    assert!(
        !is_expired,
        "Closing a connection must not set the expired flag"
    );
}

/// Querying expired state for an invalid handle must return an error.
#[test]
fn test_connection_is_expired_invalid_handle() {
    let client = database_driver_client();

    let invalid_handle = ConnectionHandle {
        id: 99999,
        magic: 0,
    };

    let result = client.connection_is_expired_blocking(ConnectionIsExpiredRequest {
        conn_handle: Some(invalid_handle),
    });
    assert!(result.is_err(), "Should return error for invalid handle");
}

/// A 500 server error on a query must NOT set the expired flag — 500 is a
/// transient infrastructure error, not a session-state change.
#[tokio::test]
async fn should_not_set_is_expired_on_query_500() {
    let server = MockServer::start().await;
    mount_jwt_login_success(&server).await;

    Mock::given(method("POST"))
        .and(path_regex(r"/queries/v1/query-request.*"))
        .respond_with(ResponseTemplate::new(500))
        .named("query_500")
        .mount(&server)
        .await;

    let server_uri = server.uri();
    let is_expired = tokio::task::spawn_blocking(move || {
        let client = SnowflakeTestClient::connect_integration_test(Some(&server_uri));
        let _ = client.execute_query_no_unwrap("SELECT 1");
        client.connection_is_expired_blocking().unwrap()
    })
    .await
    .unwrap();
    assert!(
        !is_expired,
        "is_expired must stay false after a 500 server error"
    );
}

/// GS 390111 (session_gone) returned during token refresh must NOT set the expired
/// flag — the master token is still valid; the session simply no longer exists on the
/// server side. Full re-auth is required, but the reason is different from 390114.
#[tokio::test]
async fn should_not_set_is_expired_when_token_request_returns_session_gone() {
    let server = MockServer::start().await;
    mount_jwt_login_success(&server).await;

    Mock::given(method("POST"))
        .and(path_regex(r"/queries/v1/query-request.*"))
        .respond_with(ResponseTemplate::new(401))
        .named("query_401")
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/session/token-request"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": false,
            "code": "390111",
            "message": "Session no longer exists. New login required to continue using Snowflake."
        })))
        .named("refresh_390111")
        .mount(&server)
        .await;

    let server_uri = server.uri();
    let is_expired = tokio::task::spawn_blocking(move || {
        let client = SnowflakeTestClient::connect_integration_test(Some(&server_uri));
        let _ = client.execute_query_no_unwrap("SELECT 1");
        client.connection_is_expired_blocking().unwrap()
    })
    .await
    .unwrap();
    assert!(
        !is_expired,
        "is_expired must stay false when token refresh returns 390111 (session_gone)"
    );
}

/// A successful token refresh must NOT set the expired flag. The session is
/// renewed and the connection is still usable (the subsequent query failing with
/// a 500 is unrelated to session state).
#[tokio::test]
async fn should_not_set_is_expired_on_successful_token_refresh() {
    let server = MockServer::start().await;
    mount_jwt_login_success(&server).await;

    // First query returns 401 once → triggers the refresh path
    Mock::given(method("POST"))
        .and(path_regex(r"/queries/v1/query-request.*"))
        .respond_with(ResponseTemplate::new(401))
        .up_to_n_times(1)
        .named("query_first_401")
        .mount(&server)
        .await;

    // After the refresh, any further query attempts get 500 (server error, not
    // a session problem — prevents the test from looping)
    Mock::given(method("POST"))
        .and(path_regex(r"/queries/v1/query-request.*"))
        .respond_with(ResponseTemplate::new(500))
        .named("query_after_refresh_500")
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/session/token-request"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "data": {
                "sessionToken": "new-session-token",
                "masterToken": "new-master-token",
                "sessionId": 12345
            }
        })))
        .named("refresh_success")
        .mount(&server)
        .await;

    let server_uri = server.uri();
    let is_expired = tokio::task::spawn_blocking(move || {
        let client = SnowflakeTestClient::connect_integration_test(Some(&server_uri));
        let _ = client.execute_query_no_unwrap("SELECT 1");
        client.connection_is_expired_blocking().unwrap()
    })
    .await
    .unwrap();
    assert!(
        !is_expired,
        "is_expired must stay false after a successful token refresh"
    );
}

/// When a query triggers a session-token refresh and the token-request endpoint
/// returns GS 390114 (master token expired), the connection must be marked
/// expired: the master token can never be renewed. Proves the refresh path
/// preserves the 390114 discriminant and reaches the `is_master_token_expired`
/// flag, matching the query-response path.
#[tokio::test]
async fn should_set_is_expired_when_refresh_returns_390114() {
    let server = MockServer::start().await;
    mount_jwt_login_success(&server).await;

    // Every query gets 401 → triggers a session-token refresh.
    Mock::given(method("POST"))
        .and(path_regex(r"/queries/v1/query-request.*"))
        .respond_with(ResponseTemplate::new(401))
        .named("query_401")
        .mount(&server)
        .await;

    // The refresh (token-request) endpoint returns GS 390114: master token expired.
    Mock::given(method("POST"))
        .and(path("/session/token-request"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": false,
            "code": "390114",
            "message": "Master token has expired. The session is no longer active."
        })))
        .named("refresh_390114")
        .mount(&server)
        .await;

    let server_uri = server.uri();
    // Run connect, the (failing) query, and the is_expired check on one blocking
    // thread that owns `client`: handle registries are per client instance and
    // SnowflakeTestClient releases its connection on Drop, so the check must go
    // through the owning client while it is still alive. The `_blocking` calls
    // also drive block_on internally and so must run off the tokio runtime thread.
    let is_expired = tokio::task::spawn_blocking(move || {
        let client = SnowflakeTestClient::connect_integration_test(Some(&server_uri));
        // 401 → refresh → 390114; the query must fail (guards against a vacuous
        // flag check if the mock ever let the query succeed).
        let query_result = client.execute_query_no_unwrap("SELECT 1");
        assert!(
            query_result.is_err(),
            "query should fail once the master token is reported expired"
        );
        client.connection_is_expired_blocking().unwrap()
    })
    .await
    .unwrap();

    assert!(
        is_expired,
        "is_expired must be true after the refresh endpoint returns GS 390114"
    );
}
