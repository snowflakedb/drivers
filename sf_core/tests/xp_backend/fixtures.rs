//! Request fixtures shared by the `xp_backend_mode_*` binaries.

#![allow(dead_code)] // each binary uses a different subset

use sf_core::config::rest_parameters::test_fixtures::test_client_info;
use sf_core::config::rest_parameters::{LoginMethod, LoginParameters, QueryParameters};
use sf_core::sensitive::SensitiveString;

/// Password used in [`login_parameters`]. Tests assert this never reaches the
/// host, so it is a distinctive literal rather than something like "password".
pub const SECRET_PASSWORD: &str = "correct-horse-battery-staple-9F3A";

pub fn clear_running_inside_xp() {
    // SAFETY: this binary is single-threaded until the driver is constructed.
    unsafe {
        std::env::remove_var(sf_core::env_vars::SNOWFLAKE_RUNNING_INSIDE_XP);
    }
}

pub fn set_running_inside_xp() {
    // SAFETY: this binary is single-threaded until the driver is constructed.
    unsafe {
        std::env::set_var(sf_core::env_vars::SNOWFLAKE_RUNNING_INSIDE_XP, "1");
    }
}

/// Unroutable on purpose: any test that accidentally takes the HTTP path should
/// fail visibly rather than reaching a real endpoint.
pub const UNREACHABLE_SERVER_URL: &str = "http://127.0.0.1:1";

pub fn query_parameters() -> QueryParameters {
    QueryParameters {
        server_url: UNREACHABLE_SERVER_URL.to_string(),
        client_info: test_client_info(),
        log_max_query_length: 80,
        log_query_text: false,
        log_query_parameters: false,
        include_retry_reason: false,
    }
}

pub fn login_parameters() -> LoginParameters {
    LoginParameters {
        account_name: "testaccount".to_string(),
        login_method: LoginMethod::Password {
            username: "SPUSER".to_string(),
            password: SensitiveString::from(SECRET_PASSWORD.to_string()),
            passcode_in_password: false,
            passcode: None,
        },
        server_url: UNREACHABLE_SERVER_URL.to_string(),
        database: Some("TESTDB".to_string()),
        schema: Some("PUBLIC".to_string()),
        warehouse: Some("TESTWH".to_string()),
        role: Some("TESTROLE".to_string()),
        secondary_roles: None,
        client_info: test_client_info(),
        session_parameters: None,
        spcs_token: None,
        disable_parallel_user_prompt: true,
        validate_session_token: true,
        browser_opener: None,
    }
}

/// A `select 1` response in the shape GS returns and XP's
/// `_snowflake.execute_sql` already forwards.
pub const SELECT_ONE_RESPONSE: &str = r#"{
    "data": {
        "rowtype": [
            {"name": "1", "type": "fixed", "length": null, "scale": 0,
             "precision": 1, "nullable": false}
        ],
        "rowset": [["1"]],
        "total": 1,
        "queryId": "01b2c3d4-0000-0000-0000-000000000001",
        "queryResultFormat": "json"
    },
    "code": null,
    "message": null,
    "success": true
}"#;

/// A monitoring-endpoint body for a still-running query, in the shape the
/// driver's own HTTP path parses. `errorCode: 0` is deliberate: Snowflake sends
/// it as an integer zero for "no error", and it must not read as error code 0.
pub const MONITORING_RUNNING_RESPONSE: &str = r#"{
    "data": {
        "queries": [
            {
                "id": "01b2c3d4-0000-0000-0000-000000000001",
                "status": "RUNNING",
                "state": "RUNNING",
                "errorCode": 0,
                "errorMessage": null,
                "startTime": 1750000000000,
                "endTime": 0,
                "totalDuration": 1200,
                "sessionId": 987654321,
                "sqlText": "select 1",
                "warehouseId": 42,
                "warehouseName": "TESTWH",
                "warehouseExternalSize": "X-Small",
                "warehouseServerType": "STANDARD"
            }
        ]
    },
    "code": null,
    "message": null,
    "success": true
}"#;

/// A session as a host inside an already-authenticated session would report it.
pub const SESSION_INFO: &str = r#"{
    "session_token": "host-session-token",
    "master_token": "host-master-token",
    "session_id": 987654321,
    "session_parameters": {"AUTOCOMMIT": "true", "TIMEZONE": "UTC"},
    "database_name": "TESTDB",
    "schema_name": "PUBLIC",
    "warehouse_name": "TESTWH",
    "role_name": "TESTROLE",
    "server_version": "9.1.0"
}"#;
