//! Integration tests for process-global prompt-lock serialization.
//!
//! These tests verify that when `clientStoreTemporaryCredential=true` and
//! `DISABLE_PARALLEL_USER_PROMPT=true` (the default), concurrent connections
//! sharing the same [`sf_core::token_cache::CacheKey`] (idp, snowflake, username,
//! role, and token_type) produce only one interactive authentication request while
//! all connections ultimately succeed.
//!
//! Feature: parallel_user_prompt_locking.feature

use std::io::{Read, Write};
use std::sync::Arc;
use std::time::Duration;

use crate::common::mocks::{external_browser, mfa, oauth};
use crate::common::snowflake_test_client::SnowflakeTestClient;
use crate::common::tls_proxy::MockServerWithTls;
use sf_core::protobuf::apis::database_driver_v1::{
    DatabaseDriverClient, DriverProviders, database_driver_client_with,
};
use sf_core::rest::snowflake::prompt_lock::PromptLockMap;
use sf_core::token_cache::{
    CacheKey, KeyringTokenCache, TokenCache, TokenType, normalize_identifier, normalize_url,
};

// =============================================================================
// Helpers
// =============================================================================

/// Simulate a browser callback delivering `token` to the loopback listener
/// port recorded in the n-th authenticator-request body (0-indexed).
///
/// When multiple concurrent connections each bind their own loopback port,
/// each one embeds its `BROWSER_MODE_REDIRECT_PORT` in the authenticator-
/// request it sends.  Using the index lets the watcher threads route each
/// callback to the correct connection's listener rather than always hitting
/// the first one.
fn simulate_browser_callback_nth(mock: &MockServerWithTls, token: &str, n: usize) {
    let requests = mock.received_requests();
    let authn_req = requests
        .iter()
        .filter(|r| r.url.path().contains("authenticator-request"))
        .nth(n)
        .unwrap_or_else(|| panic!("No authenticator-request #{n} captured"));

    let body: serde_json::Value =
        serde_json::from_slice(&authn_req.body).expect("Request body is not valid JSON");
    let port: u16 = body["data"]["BROWSER_MODE_REDIRECT_PORT"]
        .as_str()
        .expect("BROWSER_MODE_REDIRECT_PORT not found")
        .parse()
        .expect("BROWSER_MODE_REDIRECT_PORT is not a valid port number");

    let mut stream = std::net::TcpStream::connect(format!("127.0.0.1:{port}"))
        .expect("Failed to connect to callback listener");
    let request = format!("GET /?token={token} HTTP/1.1\r\nHost: localhost\r\n\r\n");
    stream
        .write_all(request.as_bytes())
        .expect("Failed to write to callback listener");
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .expect("Failed to read response from callback listener");
    let resp = String::from_utf8_lossy(&response);
    assert!(resp.contains("200 OK"), "Expected 200 OK, got: {resp}");
}

/// Convenience wrapper: simulate a browser callback for the first (and only)
/// authenticator-request in flight.
fn simulate_browser_callback(mock: &MockServerWithTls, token: &str) {
    simulate_browser_callback_nth(mock, token, 0);
}

/// Build a fresh `SnowflakeTestClient` pointing at `mock` with EB auth + caching.
fn eb_client_with_caching(mock: &MockServerWithTls, user: &str) -> SnowflakeTestClient {
    let c = SnowflakeTestClient::with_int_tests_params(Some(&mock.http_url()));
    c.set_connection_option("authenticator", "EXTERNALBROWSER");
    c.set_connection_option("user", user);
    c.set_connection_option("authentication_timeout", "30");
    c.set_connection_option("client_store_temporary_credential", "true");
    c
}

/// Build a fresh `SnowflakeTestClient` pointing at `mock` with EB auth, no caching.
fn eb_client_no_caching(mock: &MockServerWithTls, user: &str) -> SnowflakeTestClient {
    let c = SnowflakeTestClient::with_int_tests_params(Some(&mock.http_url()));
    c.set_connection_option("authenticator", "EXTERNALBROWSER");
    c.set_connection_option("user", user);
    c.set_connection_option("authentication_timeout", "30");
    // client_store_temporary_credential intentionally NOT set (defaults to false)
    c
}

/// Allocate a fresh, empty `PromptLockMap` wrapped in an `Arc`.
///
/// Tests that exercise the "one-prompt" serialization path should create one
/// shared map and pass `Arc::clone` of it to every client that must share the
/// same lock space.  Each `SnowflakeTestClient` creates its own
/// `DatabaseDriverV1` internally, so without a shared map the locks would be
/// independent and serialization would not occur.
fn make_shared_locks() -> Arc<PromptLockMap> {
    Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()))
}

/// Create a `DatabaseDriverClient` whose backing driver uses `shared_locks`
/// as its prompt-lock map.
fn make_db_client_with_shared_locks(shared_locks: Arc<PromptLockMap>) -> DatabaseDriverClient {
    database_driver_client_with(DriverProviders {
        prompt_locks: Some(shared_locks),
        ..Default::default()
    })
}

/// Build a `SnowflakeTestClient` with EB auth + caching, backed by the
/// provided shared lock map so that multiple clients block on the same lock.
fn eb_client_with_shared_locks(
    mock: &MockServerWithTls,
    user: &str,
    shared_locks: Arc<PromptLockMap>,
) -> SnowflakeTestClient {
    let db_client = make_db_client_with_shared_locks(shared_locks);
    let c =
        SnowflakeTestClient::with_int_tests_params_and_client(Some(&mock.http_url()), db_client);
    c.set_connection_option("authenticator", "EXTERNALBROWSER");
    c.set_connection_option("user", user);
    c.set_connection_option("authentication_timeout", "30");
    c.set_connection_option("client_store_temporary_credential", "true");
    c
}

// =============================================================================
// Scenario: should show only one external browser prompt when multiple
//           connections authenticate concurrently
// =============================================================================

#[test]
fn should_show_only_one_external_browser_prompt_when_multiple_connections_authenticate_concurrently()
 {
    // Given clientStoreTemporaryCredential is enabled and DISABLE_PARALLEL_USER_PROMPT is true
    let user = "eb_lock_concurrent";
    let mock = MockServerWithTls::start();
    let cache = KeyringTokenCache::new().expect("token cache should be available");
    // The default test parameters supply role="test_role"; the production code
    // embeds normalize_identifier(login_parameters.role) in the ID-token cache key.
    let eb_id_token_key = CacheKey {
        token_type: TokenType::IdToken,
        idp: normalize_url(&mock.http_url()),
        snowflake: normalize_url(&mock.http_url()),
        username: normalize_identifier(user),
        role: normalize_identifier("test_role"),
    };
    // Ensure no leftover token from a previous run
    let _ = cache.remove_token(&eb_id_token_key);

    // And Wiremock returns valid ssoUrl and proofKey for authenticator-request
    mock.mount(external_browser::authenticator_request(
        "https://idp.example.com/sso",
        "proof_key_lock",
    ));
    // Connection 1 uses the full EB flow; its response includes idToken so the driver caches it.
    // Connection 2 (after the lock) finds the cached idToken and logs in with AUTHENTICATOR=ID_TOKEN.
    // And Login endpoint returns success
    mock.mount(external_browser::login_success_with_id_token_in_response());
    mock.mount(external_browser::login_success_for_cached_id_token_flow());

    // Both clients must share the same PromptLockMap so that their concurrent
    // login attempts are serialized against the same lock entry.
    let shared_locks = make_shared_locks();
    let client1 = eb_client_with_shared_locks(&mock, user, Arc::clone(&shared_locks));
    let client2 = eb_client_with_shared_locks(&mock, user, Arc::clone(&shared_locks));

    // When Multiple connections attempt external browser login concurrently
    let mock_ref = &mock;

    let (result1, result2) = temp_env::with_var("SF_TEST_BROWSER_OPENER", Some("noop"), || {
        std::thread::scope(|s| {
            // Watcher thread: when the first authenticator-request is seen, deliver the
            // browser callback so the first connection can proceed.
            s.spawn(move || {
                for _ in 0..100 {
                    std::thread::sleep(Duration::from_millis(100));
                    let requests = mock_ref.received_requests();
                    let n = requests
                        .iter()
                        .filter(|r| r.url.path().contains("authenticator-request"))
                        .count();
                    if n >= 1 {
                        simulate_browser_callback(mock_ref, "browser_sso_token_locked");
                        return;
                    }
                }
                panic!("Timed out waiting for authenticator-request");
            });

            // Launch both connections concurrently.
            let h2 = s.spawn(|| client2.connect());
            let r1 = client1.connect();
            let r2 = h2.join().unwrap();
            (r1, r2)
        })
    });

    // Then Only one authenticator-request is sent to the server
    let requests = mock.received_requests();
    let authn_req_count = requests
        .iter()
        .filter(|r| r.url.path().contains("authenticator-request"))
        .count();
    assert_eq!(
        authn_req_count, 1,
        "Expected exactly 1 authenticator-request (one prompt), got {authn_req_count}"
    );

    // Cleanup before asserting so the cache is restored even if an assert panics.
    let _ = cache.remove_token(&eb_id_token_key);

    // And All connections succeed
    assert!(
        result1.is_ok(),
        "Connection 1 should succeed, got: {result1:?}"
    );
    assert!(
        result2.is_ok(),
        "Connection 2 should succeed, got: {result2:?}"
    );
}

// =============================================================================
// Scenario: should show only one MFA prompt when multiple connections
//           authenticate concurrently
// =============================================================================

#[test]
fn should_show_only_one_mfa_prompt_when_multiple_connections_authenticate_concurrently() {
    // Given clientStoreTemporaryCredential is enabled and DISABLE_PARALLEL_USER_PROMPT is true
    let user = "mfa_lock_concurrent";
    let mock = MockServerWithTls::start();
    let cache = KeyringTokenCache::new().expect("token cache should be available");
    let mfa_token_key = CacheKey {
        token_type: TokenType::MfaToken,
        idp: normalize_url(&mock.http_url()),
        snowflake: normalize_url(&mock.http_url()),
        username: normalize_identifier(user),
        role: String::new(),
    };
    // Ensure no leftover token from a previous run
    let _ = cache.remove_token(&mfa_token_key);

    // Connection 1 does the interactive MFA push; the response includes mfaToken which the driver caches.
    // Connection 2 (after the lock) finds the cached MFA token and logs in with TOKEN set.
    // And Wiremock returns successful login with MFA token for the first connection
    mock.mount(mfa::login_success_with_mfa_token());
    mock.mount(mfa::login_success_with_cached_token_value(
        "mock_mfa_token_from_server",
    ));

    // Both clients must share the same PromptLockMap so that their concurrent
    // login attempts are serialized against the same lock entry.
    let shared_locks = make_shared_locks();
    let client1 = {
        let db_client = make_db_client_with_shared_locks(Arc::clone(&shared_locks));
        let c = SnowflakeTestClient::with_int_tests_params_and_client(
            Some(&mock.http_url()),
            db_client,
        );
        c.set_connection_option("authenticator", "USERNAME_PASSWORD_MFA");
        c.set_connection_option("user", user);
        c.set_connection_option("password", "test_password"); // pragma: allowlist secret
        c.set_connection_option("client_store_temporary_credential", "true");
        c
    };
    let client2 = {
        let db_client = make_db_client_with_shared_locks(Arc::clone(&shared_locks));
        let c = SnowflakeTestClient::with_int_tests_params_and_client(
            Some(&mock.http_url()),
            db_client,
        );
        c.set_connection_option("authenticator", "USERNAME_PASSWORD_MFA");
        c.set_connection_option("user", user);
        c.set_connection_option("password", "test_password"); // pragma: allowlist secret
        c.set_connection_option("client_store_temporary_credential", "true");
        c
    };

    // When Multiple connections attempt username_password_mfa login concurrently
    let (result1, result2) = std::thread::scope(|s| {
        let h2 = s.spawn(|| client2.connect());
        let r1 = client1.connect();
        let r2 = h2.join().unwrap();
        (r1, r2)
    });

    // An interactive MFA login carries EXT_AUTHN_DUO_METHOD and no TOKEN.
    // Cached-token logins also use AUTHENTICATOR=USERNAME_PASSWORD_MFA but set TOKEN — exclude them.
    // Then Only one interactive MFA login-request is sent to the server
    let requests = mock.received_requests();
    let interactive_mfa_count = requests
        .iter()
        .filter(|r| {
            if !r.url.path().contains("login-request") {
                return false;
            }
            let body: serde_json::Value = serde_json::from_slice(&r.body).unwrap_or_default();
            body["data"]["AUTHENTICATOR"]
                .as_str()
                .map(|a| a == "USERNAME_PASSWORD_MFA")
                .unwrap_or(false)
                && body["data"]["TOKEN"].is_null() // cached-token logins have TOKEN set
        })
        .count();
    assert_eq!(
        interactive_mfa_count, 1,
        "Expected exactly 1 interactive MFA login (USERNAME_PASSWORD_MFA), got {interactive_mfa_count}"
    );

    // Cleanup before asserting so the cache is restored even if an assert panics.
    let _ = cache.remove_token(&mfa_token_key);

    // And All connections succeed using the cached MFA token
    assert!(
        result1.is_ok(),
        "Connection 1 should succeed, got: {result1:?}"
    );
    assert!(
        result2.is_ok(),
        "Connection 2 should succeed, got: {result2:?}"
    );
}

// =============================================================================
// Scenario: should show independent prompts when DISABLE_PARALLEL_USER_PROMPT
//           is false
// =============================================================================

#[test]
fn should_show_independent_prompts_when_disable_parallel_user_prompt_is_false() {
    // Given clientStoreTemporaryCredential is enabled and DISABLE_PARALLEL_USER_PROMPT is false
    let user = "eb_no_lock";
    let mock = MockServerWithTls::start();

    // Two authenticator-request stubs so each connection can get its own ssoUrl
    // And Wiremock returns valid ssoUrl and proofKey for each authenticator-request
    mock.mount(external_browser::authenticator_request(
        "https://idp.example.com/sso",
        "proof_key_nlock_1",
    ));
    mock.mount(external_browser::authenticator_request(
        "https://idp.example.com/sso",
        "proof_key_nlock_2",
    ));
    // And Login endpoint returns success
    mock.mount(external_browser::login_success());
    mock.mount(external_browser::login_success());

    let make_client = |mock: &MockServerWithTls| {
        let c = SnowflakeTestClient::with_int_tests_params(Some(&mock.http_url()));
        c.set_connection_option("authenticator", "EXTERNALBROWSER");
        c.set_connection_option("user", user);
        c.set_connection_option("authentication_timeout", "30");
        c.set_connection_option("client_store_temporary_credential", "true");
        c.set_connection_option("DISABLE_PARALLEL_USER_PROMPT", "false");
        c
    };

    let client1 = make_client(&mock);
    let client2 = make_client(&mock);

    // When Multiple connections attempt external browser login concurrently
    let mock_ref = &mock;
    temp_env::with_var("SF_TEST_BROWSER_OPENER", Some("noop"), || {
        std::thread::scope(|s| {
            // Deliver the second callback once two authenticator-requests have been seen.
            // Use nth(1) so we connect to connection 2's loopback port, not connection 1's.
            s.spawn(move || {
                for _ in 0..100 {
                    std::thread::sleep(Duration::from_millis(100));
                    let n = mock_ref
                        .received_requests()
                        .iter()
                        .filter(|r| r.url.path().contains("authenticator-request"))
                        .count();
                    if n >= 2 {
                        simulate_browser_callback_nth(mock_ref, "nlock_token_2", 1);
                        return;
                    }
                }
                panic!("Timed out waiting for two authenticator-requests");
            });
            // Deliver the first callback using nth(0) — connection 1's loopback port.
            s.spawn(move || {
                for _ in 0..100 {
                    std::thread::sleep(Duration::from_millis(100));
                    let n = mock_ref
                        .received_requests()
                        .iter()
                        .filter(|r| r.url.path().contains("authenticator-request"))
                        .count();
                    if n >= 1 {
                        simulate_browser_callback_nth(mock_ref, "nlock_token_1", 0);
                        return;
                    }
                }
                panic!("Timed out waiting for first authenticator-request");
            });

            let h2 = s.spawn(|| client2.connect());
            let r1 = client1.connect();
            let r2 = h2.join().unwrap();

            // Then Each connection sends its own authenticator-request to the server
            let requests = mock_ref.received_requests();
            let authn_count = requests
                .iter()
                .filter(|r| r.url.path().contains("authenticator-request"))
                .count();
            assert!(
                authn_count >= 2,
                "Expected ≥2 authenticator-requests (no locking), got {authn_count}"
            );

            // And All connections succeed independently
            assert!(r1.is_ok(), "Connection 1 should succeed, got: {r1:?}");
            assert!(r2.is_ok(), "Connection 2 should succeed, got: {r2:?}");
        });
    }); // temp_env::with_var
}

// =============================================================================
// Scenario: should show independent prompts when clientStoreTemporaryCredential
//           is false
// =============================================================================

#[test]
fn should_show_independent_prompts_when_client_store_temporary_credential_is_false() {
    // Given clientStoreTemporaryCredential is disabled and DISABLE_PARALLEL_USER_PROMPT is true
    let user = "eb_no_caching_no_lock";
    let mock = MockServerWithTls::start();

    // Two stubs because locking is not active (caching is off)
    // And Wiremock returns valid ssoUrl and proofKey for each authenticator-request
    mock.mount(external_browser::authenticator_request(
        "https://idp.example.com/sso",
        "proof_key_nocache_1",
    ));
    mock.mount(external_browser::authenticator_request(
        "https://idp.example.com/sso",
        "proof_key_nocache_2",
    ));
    // And Login endpoint returns success
    mock.mount(external_browser::login_success());
    mock.mount(external_browser::login_success());

    let client1 = eb_client_no_caching(&mock, user);
    let client2 = eb_client_no_caching(&mock, user);

    // When Multiple connections attempt external browser login concurrently
    let mock_ref = &mock;
    temp_env::with_var("SF_TEST_BROWSER_OPENER", Some("noop"), || {
        std::thread::scope(|s| {
            // nth(1) routes the second callback to connection 2's loopback port.
            s.spawn(move || {
                for _ in 0..100 {
                    std::thread::sleep(Duration::from_millis(100));
                    let n = mock_ref
                        .received_requests()
                        .iter()
                        .filter(|r| r.url.path().contains("authenticator-request"))
                        .count();
                    if n >= 2 {
                        simulate_browser_callback_nth(mock_ref, "nocache_token_2", 1);
                        return;
                    }
                }
                panic!("Timed out waiting for two authenticator-requests");
            });
            s.spawn(move || {
                for _ in 0..100 {
                    std::thread::sleep(Duration::from_millis(100));
                    let n = mock_ref
                        .received_requests()
                        .iter()
                        .filter(|r| r.url.path().contains("authenticator-request"))
                        .count();
                    if n >= 1 {
                        simulate_browser_callback_nth(mock_ref, "nocache_token_1", 0);
                        return;
                    }
                }
                panic!("Timed out waiting for first authenticator-request");
            });

            let h2 = s.spawn(|| client2.connect());
            let r1 = client1.connect();
            let r2 = h2.join().unwrap();

            // Then Each connection sends its own authenticator-request to the server
            let requests = mock_ref.received_requests();
            let authn_count = requests
                .iter()
                .filter(|r| r.url.path().contains("authenticator-request"))
                .count();
            assert!(
                authn_count >= 2,
                "Expected ≥2 authenticator-requests (caching off), got {authn_count}"
            );

            // And All connections succeed independently
            assert!(r1.is_ok(), "Connection 1 should succeed, got: {r1:?}");
            assert!(r2.is_ok(), "Connection 2 should succeed, got: {r2:?}");
        });
    }); // temp_env::with_var
}

// =============================================================================
// Scenario: should show only one OAuth authorization code IdP exchange when
//           multiple connections authenticate concurrently
// =============================================================================

#[test]
fn should_show_only_one_oauth_authorization_code_idp_exchange_when_multiple_connections_authenticate_concurrently()
 {
    // Given clientStoreTemporaryCredential is enabled and DISABLE_PARALLEL_USER_PROMPT is true
    let user = "oauth_ac_lock_concurrent";
    let mock = MockServerWithTls::start();
    let cache = KeyringTokenCache::new().expect("token cache should be available");
    let oauth_token_url = format!("{}/oauth/token-request", mock.http_url());
    // The default test parameters supply role="test_role"; the production code
    // passes login_parameters.role to run_oauth_authorization_code, which uses it
    // in the CacheKey so all cache reads/writes are role-scoped.
    let make_oauth_key = |token_type: TokenType| CacheKey {
        token_type,
        idp: normalize_url(&oauth_token_url),
        snowflake: normalize_url(&mock.http_url()),
        username: normalize_identifier(user),
        role: normalize_identifier("test_role"),
    };
    let _ = cache.remove_token(&make_oauth_key(TokenType::OAuthAccessToken));
    let _ = cache.remove_token(&make_oauth_key(TokenType::OAuthRefreshToken));

    // Note: the AC interactive leg requires a real browser redirect callback; the refresh-token
    // path is used here to keep the test self-contained while still exercising the lock code.
    // And A refresh token is seeded in the cache to bypass the interactive browser leg
    cache
        .add_token(
            &make_oauth_key(TokenType::OAuthRefreshToken),
            "rt-for-lock-test",
        )
        .expect("seed refresh token");

    // And IdP token endpoint returns a fresh access token on refresh_token exchange
    mock.mount(oauth::idp_token_endpoint_success_refresh());

    // And Snowflake login endpoint returns success for OAuth
    mock.mount(oauth::snowflake_login_success_oauth(
        "ac-access-token-refreshed",
    ));

    // Both clients must share the same PromptLockMap so that their concurrent
    // login attempts are serialized against the same lock entry.
    let shared_locks = make_shared_locks();
    let make_client = |mock: &MockServerWithTls, locks: Arc<PromptLockMap>| {
        let token_url = format!("{}/oauth/token-request", mock.http_url());
        let auth_url = format!("{}/oauth/authorize", mock.http_url());
        let db_client = make_db_client_with_shared_locks(locks);
        let c = SnowflakeTestClient::with_int_tests_params_and_client(
            Some(&mock.http_url()),
            db_client,
        );
        c.set_connection_option("authenticator", "OAUTH_AUTHORIZATION_CODE");
        c.set_connection_option("user", user);
        c.set_connection_option("oauth_client_id", "test-oauth-client-id");
        c.set_connection_option(
            "oauth_client_secret",
            "test-oauth-client-secret", // pragma: allowlist secret
        );
        c.set_connection_option("oauth_token_request_url", &token_url);
        c.set_connection_option("oauth_authorization_url", &auth_url);
        c.set_connection_option("oauth_scope", "session:role:test_role");
        c.set_connection_option("client_store_temporary_credential", "true");
        c
    };

    let client1 = make_client(&mock, Arc::clone(&shared_locks));
    let client2 = make_client(&mock, Arc::clone(&shared_locks));

    // When Multiple connections attempt OAuth authorization code login concurrently
    let (result1, result2) = std::thread::scope(|s| {
        let h2 = s.spawn(|| client2.connect());
        let r1 = client1.connect();
        let r2 = h2.join().unwrap();
        (r1, r2)
    });

    // Then Only one IdP token exchange is performed
    let requests = mock.received_requests();
    let token_exchange_count = requests
        .iter()
        .filter(|r| {
            r.url.path() == "/oauth/token-request"
                && String::from_utf8_lossy(&r.body).contains("grant_type=refresh_token")
        })
        .count();
    assert_eq!(
        token_exchange_count, 1,
        "Expected exactly 1 IdP token exchange (lock serialized), got {token_exchange_count}"
    );

    // Cleanup before asserting so the cache is restored even if an assert panics.
    let _ = cache.remove_token(&make_oauth_key(TokenType::OAuthAccessToken));
    let _ = cache.remove_token(&make_oauth_key(TokenType::OAuthRefreshToken));

    // And All connections succeed using the cached access token
    assert!(
        result1.is_ok(),
        "Connection 1 should succeed, got: {result1:?}"
    );
    assert!(
        result2.is_ok(),
        "Connection 2 should succeed, got: {result2:?}"
    );
}

#[test]
fn should_release_the_lock_when_the_first_connection_login_fails_so_the_waiting_connection_can_authenticate_independently()
 {
    // Given clientStoreTemporaryCredential is enabled and DISABLE_PARALLEL_USER_PROMPT is true
    let user = "eb_lock_fail_first";
    let mock = MockServerWithTls::start();
    let cache = KeyringTokenCache::new().expect("token cache should be available");
    let fail_first_id_key = CacheKey {
        token_type: TokenType::IdToken,
        idp: normalize_url(&mock.http_url()),
        snowflake: normalize_url(&mock.http_url()),
        username: normalize_identifier(user),
        role: normalize_identifier("test_role"),
    };
    let _ = cache.remove_token(&fail_first_id_key);

    // Both connections need their own authenticator-request: the lock serialises them but
    // each must do a full interactive flow since no token is cached after the failure.
    // And Wiremock returns valid ssoUrl and proofKey for each authenticator-request
    mock.mount(external_browser::authenticator_request(
        "https://idp.example.com/sso",
        "proof_key_fail_1",
    ));
    mock.mount(external_browser::authenticator_request(
        "https://idp.example.com/sso",
        "proof_key_fail_2",
    ));

    // Stubs are distinguished by callback token so each connection gets the right response.
    // And Login endpoint returns failure for the first connection's browser token
    let fail_token = "eb_fail_token_c1";
    mock.mount(external_browser::login_failure_with_token(fail_token));
    // And Login endpoint returns success for the second connection's browser token
    let success_token = "eb_success_token_c2";
    mock.mount(external_browser::login_success_with_token(success_token));

    let client1 = eb_client_with_caching(&mock, user);
    let client2 = eb_client_with_caching(&mock, user);

    // When Multiple connections attempt external browser login concurrently
    let mock_ref = &mock;
    let (result1, result2) = temp_env::with_var("SF_TEST_BROWSER_OPENER", Some("noop"), || {
        std::thread::scope(|s| {
            // Watcher for connection 1: deliver fail_token once the first authenticator-request
            // is seen so connection 1 can complete its EB flow (which then fails at login).
            s.spawn(move || {
                for _ in 0..100 {
                    std::thread::sleep(Duration::from_millis(100));
                    if mock_ref
                        .received_requests()
                        .iter()
                        .filter(|r| r.url.path().contains("authenticator-request"))
                        .count()
                        >= 1
                    {
                        simulate_browser_callback_nth(mock_ref, fail_token, 0);
                        return;
                    }
                }
                panic!("Timed out waiting for connection 1 authenticator-request");
            });

            // Watcher for connection 2: deliver success_token once the second authenticator-request
            // appears — this only happens after connection 1 has failed and released the lock.
            s.spawn(move || {
                for _ in 0..200 {
                    std::thread::sleep(Duration::from_millis(100));
                    if mock_ref
                        .received_requests()
                        .iter()
                        .filter(|r| r.url.path().contains("authenticator-request"))
                        .count()
                        >= 2
                    {
                        simulate_browser_callback_nth(mock_ref, success_token, 1);
                        return;
                    }
                }
                panic!(
                    "Timed out waiting for connection 2 authenticator-request after lock release"
                );
            });

            let h2 = s.spawn(|| client2.connect());
            let r1 = client1.connect();
            let r2 = h2.join().unwrap();
            (r1, r2)
        })
    });

    // Cleanup before asserting so the cache is restored even if an assert panics.
    let _ = cache.remove_token(&fail_first_id_key);

    // The connection that acquired the lock first gets the fail token → auth error.
    // The connection that acquired it second gets the success token → succeeds.
    // We do not assert which of result1/result2 is which because OS thread scheduling
    // is non-deterministic: on some platforms the spawned thread (client2) wins the
    // lock race before the calling thread (client1).
    // Then The first connection fails with an authentication error
    let num_err = [result1.is_err(), result2.is_err()]
        .iter()
        .filter(|&&x| x)
        .count();
    assert_eq!(
        num_err, 1,
        "Expected exactly 1 connection to fail (got {num_err}): r1={result1:?}, r2={result2:?}"
    );
    // And The second connection acquires the released lock and succeeds
    let num_ok = [result1.is_ok(), result2.is_ok()]
        .iter()
        .filter(|&&x| x)
        .count();
    assert_eq!(
        num_ok, 1,
        "Expected exactly 1 connection to succeed (got {num_ok}): r1={result1:?}, r2={result2:?}"
    );

    // Each connection ran its own interactive flow, serialised by the lock.
    // And Two authenticator-requests were sent to the server
    let authn_count = mock
        .received_requests()
        .iter()
        .filter(|r| r.url.path().contains("authenticator-request"))
        .count();
    assert_eq!(
        authn_count, 2,
        "Expected exactly 2 authenticator-requests (one per serialised attempt), got {authn_count}"
    );
}

// =============================================================================
// Scenario: should release the lock when the browser callback times out
//           so the waiting connection can authenticate independently
// =============================================================================

#[test]
fn should_release_the_lock_when_the_browser_callback_times_out_so_the_waiting_connection_can_authenticate_independently()
 {
    // Given clientStoreTemporaryCredential is enabled and DISABLE_PARALLEL_USER_PROMPT is true
    let user = "eb_lock_timeout_first";
    let mock = MockServerWithTls::start();
    let cache = KeyringTokenCache::new().expect("token cache should be available");
    let timeout_id_key = CacheKey {
        token_type: TokenType::IdToken,
        idp: normalize_url(&mock.http_url()),
        snowflake: normalize_url(&mock.http_url()),
        username: normalize_identifier(user),
        role: normalize_identifier("test_role"),
    };
    let _ = cache.remove_token(&timeout_id_key);

    // 5 seconds is long enough to be reliable but short enough that the test
    // does not run for too long.
    // And authentication_timeout is configured to a short duration
    let timeout_secs = "5";

    // Both connections need their own authenticator-request since no token is cached.
    // And Wiremock returns valid ssoUrl and proofKey for each authenticator-request
    mock.mount(external_browser::authenticator_request(
        "https://idp.example.com/sso",
        "proof_key_timeout_1",
    ));
    mock.mount(external_browser::authenticator_request(
        "https://idp.example.com/sso",
        "proof_key_timeout_2",
    ));

    // The connection that acquires the lock second delivers token "cb_after_timeout" and
    // logs in with it.  We use a token-specific mock so a spurious login request
    // (e.g. after an unexpected timeout path on some platforms) cannot accidentally
    // match a generic success stub.
    // And Login endpoint returns success
    let second_conn_token = "cb_after_timeout";
    mock.mount(external_browser::login_success_with_token(
        second_conn_token,
    ));

    let make_client = |mock: &MockServerWithTls| {
        let c = SnowflakeTestClient::with_int_tests_params(Some(&mock.http_url()));
        c.set_connection_option("authenticator", "EXTERNALBROWSER");
        c.set_connection_option("user", user);
        c.set_connection_option("authentication_timeout", timeout_secs);
        c.set_connection_option("client_store_temporary_credential", "true");
        c
    };
    let client1 = make_client(&mock);
    let client2 = make_client(&mock);

    // When Multiple connections attempt external browser login concurrently
    let mock_ref = &mock;
    // The connection that acquires the lock first receives no callback and times out.
    // After it releases the lock, the second connection proceeds and gets the callback.
    // And The browser callback is never delivered to the first connection
    let (result1, result2) = temp_env::with_var("SF_TEST_BROWSER_OPENER", Some("noop"), || {
        std::thread::scope(|s| {
            // Deliver a callback once the second authenticator-request appears (that is,
            // after the first lock-holder timed out and the second connection acquired
            // the lock).  The watcher waits up to 30 s (well beyond timeout_secs).
            s.spawn(move || {
                for _ in 0..300 {
                    std::thread::sleep(Duration::from_millis(100));
                    if mock_ref
                        .received_requests()
                        .iter()
                        .filter(|r| r.url.path().contains("authenticator-request"))
                        .count()
                        >= 2
                    {
                        simulate_browser_callback_nth(mock_ref, second_conn_token, 1);
                        return;
                    }
                }
                panic!("Timed out waiting for second authenticator-request after lock release");
            });

            let h2 = s.spawn(|| client2.connect());
            let r1 = client1.connect();
            let r2 = h2.join().unwrap();
            (r1, r2)
        })
    });

    // Cleanup before asserting so the cache is restored even if an assert panics.
    let _ = cache.remove_token(&timeout_id_key);

    // The connection that acquired the lock first receives no callback → times out → fails.
    // The connection that acquired the lock second receives the callback → succeeds.
    // We do not assert which of result1/result2 is which because OS thread scheduling
    // is non-deterministic: the spawned thread (client2) may win the lock race before
    // the calling thread (client1) on some platforms.
    // Then The first connection fails with a timeout error
    let num_err = [result1.is_err(), result2.is_err()]
        .iter()
        .filter(|&&x| x)
        .count();
    assert_eq!(
        num_err, 1,
        "Expected exactly 1 connection to fail (got {num_err}): r1={result1:?}, r2={result2:?}"
    );
    // And The second connection acquires the released lock and succeeds
    let num_ok = [result1.is_ok(), result2.is_ok()]
        .iter()
        .filter(|&&x| x)
        .count();
    assert_eq!(
        num_ok, 1,
        "Expected exactly 1 connection to succeed (got {num_ok}): r1={result1:?}, r2={result2:?}"
    );

    // And Two authenticator-requests were sent to the server
    let authn_count = mock
        .received_requests()
        .iter()
        .filter(|r| r.url.path().contains("authenticator-request"))
        .count();
    assert_eq!(
        authn_count, 2,
        "Expected exactly 2 authenticator-requests (one per serialised attempt), got {authn_count}"
    );
}
