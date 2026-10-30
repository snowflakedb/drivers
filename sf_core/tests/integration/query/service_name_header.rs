use crate::common::snowflake_test_client::SnowflakeTestClient;
use crate::common::tls_proxy::MockServerWithTls;
use serde_json::json;
use wiremock::matchers::{header, method, path_regex};
use wiremock::{Mock, ResponseTemplate};

fn login_success_with_service_name(service_name: &str) -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/session/v1/login-request"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "data": {
                "token": "mock_session_token",
                "masterToken": "mock_master_token",
                "sessionId": 12345,
                "validityInSeconds": 3600,
                "masterValidityInSeconds": 14400,
                "parameters": [
                    {
                        "name": "SERVICE_NAME",
                        "value": service_name
                    }
                ],
                "sessionInfo": {
                    "databaseName": "test_database",
                    "schemaName": "test_schema",
                    "warehouseName": "test_warehouse",
                    "roleName": "test_role"
                }
            }
        })))
}

fn query_returning_service_name(
    expected_service_name: &str,
    new_service_name: Option<&str>,
) -> Mock {
    let parameters = new_service_name.map(|name| {
        json!([{
            "name": "SERVICE_NAME",
            "value": name
        }])
    });

    Mock::given(method("POST"))
        .and(path_regex(r"/queries/v1/query-request.*"))
        .and(header("X-Snowflake-Service", expected_service_name))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "data": {
                "queryId": "qid-1",
                "queryResultFormat": "json",
                "rowset": [],
                "rowtype": [],
                "total": 0,
                "parameters": parameters,
            }
        })))
        .expect(1)
}

#[test]
fn query_sends_service_name_header_when_login_returns_service_name() {
    let mock = MockServerWithTls::start();
    let client = SnowflakeTestClient::with_int_tests_params(Some(&mock.http_url()));
    client.set_connection_option("password", "test_password"); // pragma: allowlist secret

    mock.mount(login_success_with_service_name("my-cluster"));
    mock.mount(query_returning_service_name("my-cluster", None));

    client.connect().expect("connect should succeed");
    client
        .execute_query_no_unwrap("SELECT 1")
        .expect("query should succeed with X-Snowflake-Service header");
}

#[test]
fn query_picks_up_service_name_changed_by_prior_query_response() {
    let mock = MockServerWithTls::start();
    let client = SnowflakeTestClient::with_int_tests_params(Some(&mock.http_url()));
    client.set_connection_option("password", "test_password"); // pragma: allowlist secret

    mock.mount(login_success_with_service_name("cluster-a"));
    mock.mount(query_returning_service_name("cluster-a", Some("cluster-b")));
    mock.mount(query_returning_service_name("cluster-b", None));

    client.connect().expect("connect should succeed");

    client
        .execute_query_no_unwrap("SELECT 1")
        .expect("first query should send X-Snowflake-Service: cluster-a");

    client
        .execute_query_no_unwrap("SELECT 2")
        .expect("second query should send X-Snowflake-Service: cluster-b");
}
