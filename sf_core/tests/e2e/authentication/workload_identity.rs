//! End-to-end tests for Workload Identity Federation (WIF) authentication.
//!
//! These tests require execution on a cloud VM / serverless runner that has
//! an actual cloud identity attached (AWS instance role, Azure Managed
//! Identity, GCP service account, or a pre-acquired OIDC token).
//!
//! Required parameters.json keys:
//!
//!   * `SNOWFLAKE_TEST_WIF_PROVIDER`           — `AWS` | `AZURE` | `GCP` | `OIDC`
//!   * `SNOWFLAKE_TEST_WIF_ACCOUNT`            — Snowflake account identifier
//!   * `SNOWFLAKE_TEST_WIF_USER`               — Snowflake user (optional for WIF)
//!
//! Optional parameters.json keys:
//!
//!   * `SNOWFLAKE_TEST_WIF_ENTRA_RESOURCE`     — Azure Entra resource URI (AZURE only)
//!   * `SNOWFLAKE_TEST_WIF_IMPERSONATION_PATH` — comma-separated impersonation chain
//!
//! Scenario step text mirrors `tests/definitions/shared/authentication/workload_identity.feature`
//! so a single Gherkin definition validates both sf_core and ODBC test methods.

use crate::common::snowflake_test_client::SnowflakeTestClient;
use sf_core::config::rest_parameters::WifProvider;

// ---------------------------------------------------------------------------
// Happy-path scenarios
// ---------------------------------------------------------------------------

#[test]
fn should_authenticate_with_cloud_identity_using_wif() {
    //Given Authentication is set to WORKLOAD_IDENTITY and WORKLOAD_IDENTITY_PROVIDER is configured
    let client = build_wif_client();
    let Some(provider) = require_wif_provider(&client) else {
        return;
    };
    client.set_connection_option("authenticator", "WORKLOAD_IDENTITY");
    client.set_connection_option("workload_identity_provider", &provider);
    apply_optional_wif_params(&client);

    //When Trying to Connect
    let result = client.connect();

    //Then Login is successful and a simple query can be executed
    client.verify_simple_query(result);
}

#[test]
fn should_authenticate_with_wif_using_lowercase_authenticator() {
    //Given Authentication is set to workload_identity (lowercase) and a valid provider is configured
    let client = build_wif_client();
    let Some(provider) = require_wif_provider(&client) else {
        return;
    };
    client.set_connection_option("authenticator", "workload_identity");
    client.set_connection_option("workload_identity_provider", &provider);
    apply_optional_wif_params(&client);

    //When Trying to Connect
    let result = client.connect();

    //Then Login is successful and a simple query can be executed
    client.verify_simple_query(result);
}

#[test]
fn should_authenticate_with_wif_using_impersonation() {
    //Given Authentication is set to WORKLOAD_IDENTITY and WORKLOAD_IDENTITY_IMPERSONATION_PATH is configured
    let client = build_wif_client();
    let Some(provider) = require_wif_provider(&client) else {
        return;
    };
    let impersonation_path = match client.parameters.wif_impersonation_path.clone() {
        Some(p) if !p.is_empty() => p,
        _ => {
            println!(
                "Skipping wif_should_authenticate_with_impersonation: SNOWFLAKE_TEST_WIF_IMPERSONATION_PATH not set"
            );
            return;
        }
    };
    client.set_connection_option("authenticator", "WORKLOAD_IDENTITY");
    client.set_connection_option("workload_identity_provider", &provider);
    client.set_connection_option("workload_identity_impersonation_path", &impersonation_path);

    //When Trying to Connect
    let result = client.connect();

    //Then Login is successful and a simple query can be executed
    client.verify_simple_query(result);
}

#[test]
fn should_authenticate_aws_wif_using_pre_signed_get_caller_identity_by_default() {
    //Given Authentication is set to WORKLOAD_IDENTITY and WORKLOAD_IDENTITY_PROVIDER is AWS
    let client = build_wif_client();
    let Some(provider) = require_aws_provider(&client) else {
        return;
    };
    //And SNOWFLAKE_ENABLE_AWS_WIF_OUTBOUND_TOKEN is not set
    temp_env::with_var_unset("SNOWFLAKE_ENABLE_AWS_WIF_OUTBOUND_TOKEN", || {
        client.set_connection_option("authenticator", "WORKLOAD_IDENTITY");
        client.set_connection_option("workload_identity_provider", &provider);

        //When Trying to Connect
        let result = client.connect();

        //Then Login is successful and a simple query can be executed
        client.verify_simple_query(result);
    });
}

#[test]
fn should_authenticate_aws_wif_using_get_web_identity_token_when_opted_in() {
    //Given Authentication is set to WORKLOAD_IDENTITY and WORKLOAD_IDENTITY_PROVIDER is AWS
    let client = build_wif_client();
    let Some(provider) = require_aws_provider(&client) else {
        return;
    };
    //And SNOWFLAKE_ENABLE_AWS_WIF_OUTBOUND_TOKEN is set to true
    temp_env::with_var(
        "SNOWFLAKE_ENABLE_AWS_WIF_OUTBOUND_TOKEN",
        Some("true"),
        || {
            client.set_connection_option("authenticator", "WORKLOAD_IDENTITY");
            client.set_connection_option("workload_identity_provider", &provider);

            //When Trying to Connect
            let result = client.connect();

            //Then Login is successful and a simple query can be executed
            client.verify_simple_query(result);
        },
    );
}

#[test]
fn should_authenticate_aws_wif_using_get_web_identity_token_when_workload_identity_aws_use_outbound_token_is_true()
 {
    //Given Authentication is set to WORKLOAD_IDENTITY and WORKLOAD_IDENTITY_PROVIDER is AWS
    let client = build_wif_client();
    let Some(provider) = require_aws_provider(&client) else {
        return;
    };
    //And SNOWFLAKE_ENABLE_AWS_WIF_OUTBOUND_TOKEN is not set
    temp_env::with_var_unset("SNOWFLAKE_ENABLE_AWS_WIF_OUTBOUND_TOKEN", || {
        client.set_connection_option("authenticator", "WORKLOAD_IDENTITY");
        client.set_connection_option("workload_identity_provider", &provider);
        //And workload_identity_aws_use_outbound_token is set to true
        client.set_connection_option_bool("workload_identity_aws_use_outbound_token", true);

        //When Trying to Connect
        let result = client.connect();

        //Then Login is successful and a simple query can be executed
        client.verify_simple_query(result);
    });
}

// ---------------------------------------------------------------------------
// Failure scenarios
// ---------------------------------------------------------------------------

#[test]
fn should_fail_workload_identity_when_provider_is_missing() {
    //Given Authentication is set to WORKLOAD_IDENTITY but WORKLOAD_IDENTITY_PROVIDER is absent
    let client = SnowflakeTestClient::with_default_params();
    client.set_connection_option("authenticator", "WORKLOAD_IDENTITY");
    // Provider deliberately NOT set.

    //When Trying to Connect
    let result = client.connect();

    //Then Connection fails with a missing-parameter error citing workload_identity_provider
    client.assert_missing_parameter_error(result);
}

#[test]
fn should_fail_workload_identity_when_provider_is_an_invalid_value() {
    //Given Authentication is set to WORKLOAD_IDENTITY and WORKLOAD_IDENTITY_PROVIDER is an invalid value
    let client = SnowflakeTestClient::with_default_params();
    client.set_connection_option("authenticator", "WORKLOAD_IDENTITY");
    client.set_connection_option("workload_identity_provider", "INVALID_CLOUD");

    //When Trying to Connect
    let result = client.connect();

    //Then Connection fails with an invalid-parameter error citing workload_identity_provider
    client.assert_invalid_parameter_error(result, "workload_identity_provider");
}

#[test]
fn should_fail_oidc_wif_when_token_is_missing() {
    //Given Authentication is set to WORKLOAD_IDENTITY and WORKLOAD_IDENTITY_PROVIDER is OIDC but token is absent
    let client = SnowflakeTestClient::with_default_params();
    client.set_connection_option("authenticator", "WORKLOAD_IDENTITY");
    client.set_connection_option("workload_identity_provider", "OIDC");

    //When Trying to Connect
    let result = client.connect();

    //Then Connection fails with a missing-parameter error citing token
    client.assert_missing_parameter_error(result);
}

#[test]
fn should_fail_oidc_wif_when_token_is_malformed() {
    //Given Authentication is set to WORKLOAD_IDENTITY and WORKLOAD_IDENTITY_PROVIDER is OIDC
    let client = SnowflakeTestClient::with_default_params();
    client.set_connection_option("authenticator", "WORKLOAD_IDENTITY");
    client.set_connection_option("workload_identity_provider", "OIDC");
    //And Token is set to a malformed value that is not a valid JWT
    client.set_connection_option("token", "not-a-valid-jwt");

    //When Trying to Connect
    let result = client.connect();

    //Then Connection fails with an attestation error indicating a malformed token
    let error_msg = result.expect_err("Expected error for malformed OIDC token");
    assert!(
        error_msg.contains("not a valid JWT") || error_msg.contains("malformed"),
        "Error message should indicate a malformed token: {error_msg}"
    );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Builds a WIF-ready test client using WIF-specific account / user params
/// when present, falling back to the default testconnection parameters.
fn build_wif_client() -> SnowflakeTestClient {
    let client = SnowflakeTestClient::with_default_params();
    if let Some(account) = client.parameters.wif_account.clone()
        && !account.is_empty()
    {
        client.set_connection_option("account", &account);
    }
    if let Some(user) = client.parameters.wif_user.clone()
        && !user.is_empty()
    {
        client.set_connection_option("user", &user);
    }
    client
}

/// Returns the configured WIF provider, or `None` when the test should skip.
fn require_wif_provider(client: &SnowflakeTestClient) -> Option<String> {
    let provider = match client.parameters.wif_provider.clone() {
        Some(p) if !p.is_empty() => p,
        _ => {
            println!("Skipping: SNOWFLAKE_TEST_WIF_PROVIDER not set in parameters.json");
            return None;
        }
    };
    if WifProvider::parse_str(&provider).is_none() {
        println!(
            "Skipping: workload_identity_provider '{provider}' is not supported by this build \
             (allowed: {})",
            WifProvider::allowed_values()
        );
        return None;
    }
    Some(provider)
}

/// Returns the provider when it is AWS, otherwise skips.
fn require_aws_provider(client: &SnowflakeTestClient) -> Option<String> {
    match client.parameters.wif_provider.clone() {
        Some(p) if p.eq_ignore_ascii_case("AWS") => Some(p),
        Some(_) => {
            println!("Skipping: test requires SNOWFLAKE_TEST_WIF_PROVIDER=AWS");
            None
        }
        None => {
            println!("Skipping: SNOWFLAKE_TEST_WIF_PROVIDER not set in parameters.json");
            None
        }
    }
}

/// Forwards optional WIF parameters when they are present in parameters.json.
fn apply_optional_wif_params(client: &SnowflakeTestClient) {
    if let Some(ref r) = client.parameters.wif_entra_resource.clone()
        && !r.is_empty()
    {
        client.set_connection_option("workload_identity_entra_resource", r);
    }
}
