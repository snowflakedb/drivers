//! Logins on one driver reuse the REST client, and its idle sockets, while
//! their TLS, proxy, and connect-timeout settings stay equal.
//!
//! [`CountingProxy`] counts TCP connections in front of the mock server.
//! Each login, logout, and telemetry request is HTTP/1.1, so a reused client
//! sends the next session's requests on the socket the previous session left
//! idle, and a new client opens its own. An idle socket may still close on
//! some Linux runners, which adds an accept without a new client.

use serde_json::json;
use sf_core::protobuf::apis::database_driver_v1::DriverProviders;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::common::counting_proxy::CountingProxy;
use crate::common::mocks::password;
use crate::common::snowflake_test_client::SnowflakeTestClient;

const LOGINS: usize = 3;

type LoginOptions = fn(usize) -> &'static [(&'static str, &'static str)];

async fn snowflake_behind_counting_proxy() -> (MockServer, CountingProxy) {
    let server = MockServer::start().await;
    password::login_success().mount(&server).await;
    Mock::given(method("POST"))
        .and(path("/session"))
        .and(query_param("delete", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true})))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/telemetry/send"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true})))
        .mount(&server)
        .await;
    let proxy = CountingProxy::start(*server.address()).await;
    (server, proxy)
}

/// Connects and closes [`LOGINS`] sessions in turn on one driver. Login `n`
/// sets the connection options `options(n)` returns.
async fn sequential_logins(
    proxy: &CountingProxy,
    disable_cache: bool,
    options: LoginOptions,
) -> usize {
    let uri = proxy.uri();
    tokio::task::spawn_blocking(move || {
        let providers = DriverProviders {
            disable_rest_client_cache: Some(disable_cache),
            ..Default::default()
        };
        let mut client = SnowflakeTestClient::with_int_tests_params_using(Some(&uri), providers);
        for login in 0..LOGINS {
            if login > 0 {
                client.open_another_connection();
            }
            client.set_connection_option("password", "test_password"); // pragma: allowlist secret
            client.set_connection_option("disable_platform_detection", "true");
            for (name, value) in options(login) {
                client.set_connection_option(name, value);
            }
            client.connect().expect("login succeeds");
            client.connection_close_blocking().expect("close succeeds");
        }
    })
    .await
    .expect("login task");
    proxy.accept_count()
}

#[tokio::test(flavor = "multi_thread")]
async fn logins_with_equal_settings_share_one_client() {
    let (_server, proxy) = snowflake_behind_counting_proxy().await;
    let accepts = sequential_logins(&proxy, false, |_| &[]).await;
    assert!(
        accepts < LOGINS,
        "{LOGINS} logins with equal settings must reuse one client; a client \
         per login opens at least {LOGINS} connections. got {accepts}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn disabled_cache_builds_a_client_per_login() {
    let (_server, proxy) = snowflake_behind_counting_proxy().await;
    let accepts = sequential_logins(&proxy, true, |_| &[]).await;
    assert!(
        accepts >= LOGINS,
        "with the cache off each login builds its own client and opens its \
         own connection. got {accepts}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn changed_settings_replace_the_client() {
    let (_server, proxy) = snowflake_behind_counting_proxy().await;
    let accepts = sequential_logins(&proxy, false, |login| {
        if login % 2 == 0 {
            &[("tls_prefer_post_quantum", "false")]
        } else {
            &[("tls_prefer_post_quantum", "true")]
        }
    })
    .await;
    assert!(
        accepts >= LOGINS,
        "alternating the key-share preference changes the spec on every login, \
         so each login builds a new client. got {accepts}"
    );
}
