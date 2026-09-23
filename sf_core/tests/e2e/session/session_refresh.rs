//! E2E tests for session token management and refresh.

use crate::common::arrow_result_helper::ArrowResultHelper;
use crate::common::config::{get_parameters, setup_logging};
use crate::common::private_key_helper;
use crate::common::snowflake_test_client::{SnowflakeTestClient, unwrap_single_query_id};
use sf_core::config::rest_parameters::test_fixtures::test_client_info;
use sf_core::config::rest_parameters::{LoginMethod, LoginParameters};
use sf_core::rest::snowflake::{refresh_session, snowflake_login_with_client};
use sf_core::sensitive::SensitiveString;
use sf_core::tls::client::create_tls_client_with_config;
use sf_core::tls::config::TlsConfig;

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
        let query_id = unwrap_single_query_id(&result);
        let rs = client.get_result_set(&stmt, &query_id);

        // Then each query should succeed with the correct result
        let mut helper = ArrowResultHelper::from_result(rs);
        let rows = helper.transform_into_array::<i64>().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0][0], i as i64);
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
        let query_id = unwrap_single_query_id(&result);
        let rs = client.get_result_set(&stmt, &query_id);

        // Then each query should succeed
        let mut helper = ArrowResultHelper::from_result(rs);
        let rows = helper.transform_into_array::<i64>().unwrap();
        assert_eq!(rows[0][0], i as i64);

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
        let private_key_pem = private_key_helper::get_private_key_pem_from_parameters(&parameters)
            .expect("Failed to read private key");

        let server_url = parameters
            .get_server_url()
            .expect("server_url or host required");

        let client_info = test_client_info();

        let private_key = SensitiveString::from(private_key_pem);

        let login_parameters = LoginParameters {
            server_url: server_url.clone(),
            account_name: parameters.account_name.clone().expect("account required"),
            login_method: LoginMethod::PrivateKey {
                username: parameters.user.clone().expect("user required"),
                private_key,
                passphrase: parameters
                    .private_key_password
                    .clone()
                    .map(SensitiveString::from),
            },
            database: parameters.database.clone(),
            schema: parameters.schema.clone(),
            warehouse: parameters.warehouse(),
            role: parameters.role.clone(),
            secondary_roles: None,
            client_info: client_info.clone(),
            session_parameters: None,
            spcs_token: None,
            disable_parallel_user_prompt: false,
            validate_session_token: true,
            browser_opener: None,
        };

        let http_client = create_tls_client_with_config(
            TlsConfig::insecure(),
            sf_core::crl::CrlWorker::shared_lazy(),
        )
        .expect("Failed to create HTTP client");

        // When we login and immediately call refresh
        let policy = sf_core::config::retry::RetryPolicy::default();
        let login_result = snowflake_login_with_client(
            &http_client,
            &login_parameters,
            None,
            None,
            None,
            &policy,
            None,
            None,
            sf_core::crl::CrlWorker::shared_lazy(),
        )
        .await
        .expect("Login should succeed");

        let original_session_token = login_result.tokens.session_token.clone();

        let refreshed_tokens = refresh_session(
            &http_client,
            &server_url,
            &client_info,
            &login_result.tokens,
        )
        .await
        .expect("Proactive refresh should succeed");

        // Then we should get new tokens that differ from the original
        assert_ne!(
            refreshed_tokens.session_token.reveal(),
            original_session_token.reveal(),
            "Refreshed session token should be different from original"
        );
        assert!(
            !refreshed_tokens.session_token.reveal().is_empty(),
            "New session token should not be empty"
        );
        assert!(
            !refreshed_tokens.master_token.reveal().is_empty(),
            "New master token should not be empty"
        );
    });
}

#[test]
fn should_report_token_expiry_as_wall_clock_milliseconds() {
    // Given Snowflake client is logged in
    let client = SnowflakeTestClient::connect_with_default_auth();
    let now_ms = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_millis(),
    )
    .expect("epoch milliseconds should fit in i64");

    // When connection info is read without asking for the master token
    let info = client
        .connection_get_info_blocking(false)
        .expect("get_info should succeed");

    // Then both expiries are reported, though the master token is withheld
    assert!(
        info.master_token.is_none(),
        "master token must stay withheld when include_master_token is false"
    );
    let session_expiry = info
        .session_token_expires_at_ms
        .expect("session token expiry should be reported");
    let master_expiry = info
        .master_token_expires_at_ms
        .expect("master token expiry should be reported");

    const ONE_DAY_MS: i64 = 24 * 60 * 60 * 1000;
    assert!(
        (now_ms..now_ms + ONE_DAY_MS).contains(&session_expiry),
        "session expiry {session_expiry} should be epoch ms within a day of {now_ms}"
    );
    assert!(
        (now_ms..now_ms + ONE_DAY_MS).contains(&master_expiry),
        "master expiry {master_expiry} should be epoch ms within a day of {now_ms}"
    );
    assert!(
        master_expiry > session_expiry,
        "master expiry {master_expiry} should outlive session expiry {session_expiry}"
    );
}
