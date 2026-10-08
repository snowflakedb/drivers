use serde_json::json;
use wiremock::matchers::{header, method, path_regex};
use wiremock::{Mock, ResponseTemplate};

use crate::common::snowflake_test_client::SnowflakeTestClient;
use crate::common::tls_proxy::MockServerWithTls;

struct PatWithExternalSessionFixture {
    mock: MockServerWithTls,
    client: SnowflakeTestClient,
}

impl PatWithExternalSessionFixture {
    fn new() -> Self {
        let mock = MockServerWithTls::start();
        let client = SnowflakeTestClient::with_int_tests_params(Some(&mock.http_url()));
        client.set_connection_option("authenticator", "PAT_WITH_EXTERNAL_SESSION");
        client.set_connection_option("token", "pat-token");
        client.set_connection_option("external_session_id", "ext-session-1");
        Self { mock, client }
    }
}

fn query_success() -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/queries/v1/query-request.*"))
        .and(header("Authorization", "Bearer pat-token"))
        .and(header("X-Snowflake-External-Session-ID", "ext-session-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "data": {
                    "queryId": "qid-1",
                    "queryResultFormat": "json",
                    "rowset": [],
                    "rowtype": [],
                    "total": 0,
            }
        })))
        .expect(1)
}

fn login_must_not_be_called() -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/session/v1/login-request"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({
            "success": false,
            "message": "login must not be called for PAT_WITH_EXTERNAL_SESSION",
        })))
        .expect(0)
}

fn token_request_must_not_be_called() -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/session/token-request"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({
            "success": false,
            "message": "session refresh must not be called for PAT_WITH_EXTERNAL_SESSION",
        })))
        .expect(0)
}

#[test]
fn should_connect_without_login_request_and_query_with_bearer_and_external_session_headers() {
    let fixture = PatWithExternalSessionFixture::new();
    fixture.mock.mount(login_must_not_be_called());
    fixture.mock.mount(token_request_must_not_be_called());
    fixture.mock.mount(query_success());

    let connect = fixture.client.connect();
    assert!(
        connect.is_ok(),
        "connect must succeed without a login round-trip, got: {connect:?}"
    );

    let result = fixture.client.execute_query_no_unwrap("SELECT 1");
    assert!(
        result.is_ok(),
        "query must succeed with Bearer + external-session headers, got: {result:?}"
    );

    let info = fixture
        .client
        .connection_get_info_blocking(true)
        .expect("connection info after PAT_WITH_EXTERNAL_SESSION connect");
    assert!(
        info.master_token.is_none(),
        "PAT_WITH_EXTERNAL_SESSION has no master token, got {:?}",
        info.master_token
    );
}

#[test]
fn should_reject_session_expiry_without_token_refresh() {
    let fixture = PatWithExternalSessionFixture::new();
    fixture.mock.mount(login_must_not_be_called());
    fixture.mock.mount(token_request_must_not_be_called());
    fixture.mock.mount(
        Mock::given(method("POST"))
            .and(path_regex(r"/queries/v1/query-request.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "success": false,
                "code": "390112",
                "message": "Session token is expired.",
            })))
            .expect(1),
    );

    fixture
        .client
        .connect()
        .expect("connect must succeed without login");
    let result = fixture.client.execute_query_no_unwrap("SELECT 1");
    assert!(
        result.is_err(),
        "session expiry must surface to the caller without a token-request"
    );
}
