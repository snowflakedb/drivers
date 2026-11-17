use crate::common::arrow_result_helper::ArrowResultHelper;
use crate::common::mocks::password;
use arrow::array::{Array, AsArray};
use proto_utils::ProtoError;
use serde_json::{Value, json};
use sf_core::protobuf::apis::database_driver_v1::{DatabaseDriverClient, database_driver_client};
use sf_core::protobuf::generated::database_driver_v1::*;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn connect(server: &MockServer) -> (DatabaseDriverClient, ConnectionHandle, DatabaseHandle) {
    password::login_success().mount(server).await;
    let client = database_driver_client();
    let db = client
        .database_new(DatabaseNewRequest {})
        .await
        .unwrap()
        .db_handle
        .unwrap();
    client
        .database_init(DatabaseInitRequest {
            db_handle: Some(db),
        })
        .await
        .unwrap();
    let conn = client
        .connection_new(ConnectionNewRequest {})
        .await
        .unwrap()
        .conn_handle
        .unwrap();
    client
        .connection_set_options(ConnectionSetOptionsRequest {
            conn_handle: Some(conn),
            options: [
                ("account", "test_account"),
                ("user", "test_user"),
                ("password", "test_password"), // pragma: allowlist secret
                ("host", "localhost"),
                ("protocol", "http"),
                ("server_url", &server.uri()),
            ]
            .into_iter()
            .map(|(key, value)| {
                (
                    key.to_string(),
                    ConfigSetting {
                        value: Some(config_setting::Value::StringValue(value.to_string())),
                    },
                )
            })
            .collect(),
            ..Default::default()
        })
        .await
        .unwrap();
    client
        .connection_init(ConnectionInitRequest {
            conn_handle: Some(conn),
            db_handle: Some(db),
            ..Default::default()
        })
        .await
        .unwrap();
    (client, conn, db)
}

async fn mount_show(server: &MockServer, sql: &str, response: Value) {
    Mock::given(method("POST"))
        .and(path("/queries/v1/query-request"))
        .and(body_partial_json(json!({"sqlText": sql})))
        .respond_with(ResponseTemplate::new(200).set_body_json(response))
        .mount(server)
        .await;
}

fn show_rows(columns: &[&str], rows: Value) -> Value {
    let rowtype: Vec<_> = columns
        .iter()
        .map(|name| {
            json!({
                "name": name, "type": "text", "nullable": true,
                "length": 16777216, "byteLength": 16777216,
                "precision": null, "scale": null
            })
        })
        .collect();
    json!({
        "success": true,
        "data": {
            "queryId": "mock-show-query",
            "queryResultFormat": "json",
            "rowtype": rowtype,
            "total": rows.as_array().unwrap().len(),
            "rowset": rows
        }
    })
}

async fn get_objects_with_failed_scope(code: &str, sql_state: &str) {
    let server = MockServer::start().await;
    mount_show(
        &server,
        "SHOW DATABASES IN ACCOUNT",
        show_rows(&["name"], json!([["MISSING"], ["SURVIVING"]])),
    )
    .await;
    mount_show(
        &server,
        "SHOW SCHEMAS IN DATABASE \"MISSING\"",
        json!({
            "success": false, "code": code, "message": "SHOW failed",
            "data": {"sqlState": sql_state, "queryId": "failed-show-query"}
        }),
    )
    .await;
    mount_show(
        &server,
        "SHOW SCHEMAS IN DATABASE \"SURVIVING\"",
        show_rows(&["name", "database_name"], json!([["PUBLIC", "SURVIVING"]])),
    )
    .await;
    let (client, conn, db) = connect(&server).await;

    // When one discovered database fails its scoped SHOW beside a database with rows
    let result = client
        .connection_get_objects(ConnectionGetObjectsRequest {
            conn_handle: Some(conn),
            depth: 2,
            catalog: Some("%".to_string()),
            ..Default::default()
        })
        .await;

    // Then only missing, unauthorized, or unshared scopes are empty; other errors fail
    if code == "001003" {
        let ProtoError::Application(error) = result.unwrap_err() else {
            panic!("expected a server query error");
        };
        assert_eq!(error.vendor_code, Some(1003));
        assert_eq!(error.sql_state.as_deref(), Some("42000"));
    } else {
        let handle = result.unwrap().result_set_handle.unwrap();
        let stream = client
            .result_set_get_stream(ResultSetGetStreamRequest {
                result_set_handle: Some(handle),
            })
            .await
            .unwrap();
        let mut reader = ArrowResultHelper::from_result(stream);
        let batch = reader.next_batch().expect("surviving schema row");
        assert_eq!(batch.num_rows(), 1);
        assert_eq!(batch.column(0).as_string::<i32>().value(0), "SURVIVING");
        let schemas = batch.column(1).as_list::<i64>().value(0);
        let schemas = schemas.as_struct();
        assert_eq!(schemas.len(), 1);
        assert_eq!(schemas.column(0).as_string::<i32>().value(0), "PUBLIC");
        assert!(reader.next_batch().is_none());
        client
            .result_set_release(ResultSetReleaseRequest {
                result_set_handle: Some(handle),
            })
            .await
            .unwrap();
    }
    client
        .connection_release(ConnectionReleaseRequest {
            conn_handle: Some(conn),
        })
        .await
        .unwrap();
    client
        .database_release(DatabaseReleaseRequest {
            db_handle: Some(db),
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn get_objects_missing_show_scope_preserves_other_rows_over_wire() {
    for (code, sql_state) in [
        ("002003", "42S02"),
        ("002043", "02000"),
        ("003030", "02000"),
    ] {
        get_objects_with_failed_scope(code, sql_state).await;
    }
}

#[tokio::test]
async fn get_objects_show_syntax_error_fails_over_wire() {
    get_objects_with_failed_scope("001003", "42000").await;
}
