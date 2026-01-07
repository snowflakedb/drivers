//! E2E tests for session token management and refresh.

use crate::common::arrow_result_helper::ArrowResultHelper;
use crate::common::config::{get_parameters, setup_logging};
use crate::common::private_key_helper;
use crate::common::snowflake_test_client::SnowflakeTestClient;
use sf_core::config::rest_parameters::{ClientInfo, LoginMethod, LoginParameters};
use sf_core::crl::config::CrlConfig;
use sf_core::rest::snowflake::{refresh_session, snowflake_login_with_client};
use sf_core::tls::client::create_tls_client_with_config;
use sf_core::tls::config::TlsConfig;
use std::fs;

#[test]
fn should_maintain_session_across_multiple_queries() {
    // Given Snowflake client is logged in
    let client = SnowflakeTestClient::connect_with_default_auth();

    // When we execute multiple queries
    for i in 1..=3 {
        let stmt = client.new_statement();
        let sql = format!("SELECT {} AS query_num", i);
        client.set_sql_query(&stmt, &sql);
        let result = client.execute_statement_query(&stmt);

        // Then each query should succeed with the correct result
        let mut helper = ArrowResultHelper::from_result(result);
        let rows = helper.transform_into_array::<i64>().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0][0], i as i64);

        client.release_statement(&stmt);
    }
}

#[test]
fn should_execute_queries_with_delay_between_them() {
    // Given Snowflake client is logged in
    let client = SnowflakeTestClient::connect_with_default_auth();

    // When we execute queries with delays between them
    for i in 1..=2 {
        let stmt = client.new_statement();
        let sql = format!("SELECT {} AS seq", i);
        client.set_sql_query(&stmt, &sql);
        let result = client.execute_statement_query(&stmt);

        // Then each query should succeed
        let mut helper = ArrowResultHelper::from_result(result);
        let rows = helper.transform_into_array::<i64>().unwrap();
        assert_eq!(rows[0][0], i as i64);

        client.release_statement(&stmt);

        // Short delay between queries - session should remain valid
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

#[test]
fn should_refresh_session_proactively() {
    // Given valid login credentials
    setup_logging();
    let parameters = get_parameters();

    let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");

    rt.block_on(async {
        let temp_key_file = private_key_helper::get_private_key_from_parameters(&parameters)
            .expect("Failed to create private key file");

        let server_url = parameters
            .get_server_url()
            .expect("server_url or host required");

        let client_info = ClientInfo {
            application: "sf_core_test".to_string(),
            version: "1.0.0".to_string(),
            os: std::env::consts::OS.to_string(),
            os_version: "1.0".to_string(),
            ocsp_mode: None,
            crl_config: CrlConfig::default(),
            tls_config: TlsConfig::insecure(),
        };

        let private_key =
            fs::read_to_string(temp_key_file.path()).expect("Failed to read private key file");

        let login_parameters = LoginParameters {
            server_url: server_url.clone(),
            account_name: parameters.account_name.clone().expect("account required"),
            login_method: LoginMethod::PrivateKey {
                username: parameters.user.clone().expect("user required"),
                private_key,
                passphrase: parameters.private_key_password.clone(),
            },
            database: parameters.database.clone(),
            schema: parameters.schema.clone(),
            warehouse: parameters.warehouse.clone(),
            role: parameters.role.clone(),
            client_info: client_info.clone(),
        };

        let http_client = create_tls_client_with_config(TlsConfig::insecure())
            .expect("Failed to create HTTP client");

        // When we login and immediately call refresh
        let initial_tokens = snowflake_login_with_client(&http_client, &login_parameters)
            .await
            .expect("Login should succeed");

        let original_session_token = initial_tokens.session_token.clone();

        let refreshed_tokens =
            refresh_session(&http_client, &server_url, &client_info, &initial_tokens)
                .await
                .expect("Proactive refresh should succeed");

        // Then we should get new tokens that differ from the original
        assert_ne!(
            refreshed_tokens.session_token, original_session_token,
            "Refreshed session token should be different from original"
        );
        assert!(
            !refreshed_tokens.session_token.is_empty(),
            "New session token should not be empty"
        );
        assert!(
            !refreshed_tokens.master_token.is_empty(),
            "New master token should not be empty"
        );
    });
}
