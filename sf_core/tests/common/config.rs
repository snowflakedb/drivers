use serde::{Deserialize, Serialize};
use std::fs;
use tracing::Level;
use tracing_subscriber::EnvFilter;

#[derive(Deserialize, Serialize)]
pub struct ParametersFile {
    pub testconnection: Parameters,
}

#[derive(Default, Deserialize, Serialize)]
pub struct Parameters {
    #[serde(rename = "SNOWFLAKE_TEST_ACCOUNT")]
    pub account_name: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_USER")]
    pub user: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_PASSWORD")]
    pub password: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_DATABASE")]
    pub database: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_SCHEMA")]
    pub schema: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_WAREHOUSE")]
    pub warehouse: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_WAREHOUSE_CORE")]
    pub warehouse_core: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_HOST")]
    pub host: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_ROLE")]
    pub role: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_SERVER_URL")]
    pub server_url: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_PORT")]
    pub port: Option<i64>,
    #[serde(rename = "SNOWFLAKE_TEST_PROTOCOL")]
    pub protocol: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_PRIVATE_KEY_FILE")]
    pub private_key_file: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS")]
    pub private_key_contents: Option<Vec<String>>,
    #[serde(rename = "SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD")]
    pub private_key_password: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_OKTA_URL")]
    pub okta_url: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_OKTA_USER")]
    pub okta_user: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_OKTA_PASSWORD")]
    pub okta_password: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_OKTA_ACCOUNT")]
    pub okta_account: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_OKTA_HOST")]
    pub okta_host: Option<String>,
    #[serde(default, rename = "SNOWFLAKE_TEST_OAUTH_CLIENT_ID")]
    pub oauth_client_id: Option<String>,
    #[serde(default, rename = "SNOWFLAKE_TEST_OAUTH_CLIENT_SECRET")]
    pub oauth_client_secret: Option<String>,
    #[serde(default, rename = "SNOWFLAKE_TEST_OAUTH_AUTHORIZATION_URL")]
    pub oauth_authorization_url: Option<String>,
    #[serde(default, rename = "SNOWFLAKE_TEST_OAUTH_TOKEN_REQUEST_URL")]
    pub oauth_token_request_url: Option<String>,
    #[serde(default, rename = "SNOWFLAKE_TEST_OAUTH_REDIRECT_URI")]
    pub oauth_redirect_uri: Option<String>,
    #[serde(default, rename = "SNOWFLAKE_TEST_OAUTH_SCOPE")]
    pub oauth_scope: Option<String>,
    #[serde(default, rename = "SNOWFLAKE_TEST_OAUTH_ACCESS_TOKEN")]
    pub oauth_access_token: Option<String>,
    #[serde(default, rename = "SNOWFLAKE_TEST_OKTA_OAUTH_EXTERNAL_CLIENT_ID")]
    pub okta_oauth_external_client_id: Option<String>,
    #[serde(default, rename = "SNOWFLAKE_TEST_OKTA_OAUTH_EXTERNAL_CLIENT_SECRET")]
    pub okta_oauth_external_client_secret: Option<String>,
    #[serde(default, rename = "SNOWFLAKE_TEST_OKTA_OAUTH_TOKEN_URL")]
    pub okta_oauth_token_url: Option<String>,
    #[serde(
        default,
        rename = "SNOWFLAKE_AUTH_TEST_EXTERNAL_OAUTH_OKTA_REDIRECT_URI"
    )]
    pub external_oauth_okta_redirect_uri: Option<String>,
    #[serde(default, rename = "SNOWFLAKE_AUTH_TEST_EXTERNAL_OAUTH_OKTA_AUTH_URL")]
    pub external_oauth_okta_auth_url: Option<String>,
    #[serde(
        default,
        rename = "SNOWFLAKE_AUTH_TEST_INTERNAL_OAUTH_SNOWFLAKE_WILDCARDS_CLIENT_ID"
    )]
    pub internal_oauth_snowflake_wildcards_client_id: Option<String>,
    #[serde(
        default,
        rename = "SNOWFLAKE_AUTH_TEST_INTERNAL_OAUTH_SNOWFLAKE_WILDCARDS_CLIENT_SECRET"
    )]
    pub internal_oauth_snowflake_wildcards_client_secret: Option<String>,
    #[serde(default, rename = "SNOWFLAKE_TEST_PAT")]
    pub pat: Option<String>,
    // Workload Identity Federation (WIF) parameters.
    // Populated for the cloud VM / serverless runner under which the WIF
    // identity (AWS role, Azure MI, GCP SA, or OIDC token) is available.
    #[serde(default, rename = "SNOWFLAKE_TEST_WIF_PROVIDER")]
    pub wif_provider: Option<String>,
    #[serde(default, rename = "SNOWFLAKE_TEST_WIF_ENTRA_RESOURCE")]
    pub wif_entra_resource: Option<String>,
    #[serde(default, rename = "SNOWFLAKE_TEST_WIF_IMPERSONATION_PATH")]
    pub wif_impersonation_path: Option<String>,
    #[serde(default, rename = "SNOWFLAKE_TEST_WIF_ACCOUNT")]
    pub wif_account: Option<String>,
    #[serde(default, rename = "SNOWFLAKE_TEST_WIF_USER")]
    pub wif_user: Option<String>,
    #[serde(default, rename = "SNOWFLAKE_TEST_WIF_USERNAME_IMPERSONATION")]
    pub wif_username_impersonation: Option<String>,
}

impl Parameters {
    pub fn warehouse(&self) -> Option<String> {
        self.warehouse_core
            .clone()
            .or_else(|| self.warehouse.clone())
    }

    /// Get the server URL, constructing from host if not explicitly set.
    pub fn get_server_url(&self) -> Option<String> {
        self.server_url.clone().or_else(|| {
            self.host.as_ref().map(|host| {
                let protocol = self.protocol.as_deref().unwrap_or("https");
                match self.port {
                    Some(port) => format!("{protocol}://{host}:{port}"),
                    None => format!("{protocol}://{host}"),
                }
            })
        })
    }
}

/// Parses and returns the test parameters from the configured parameter file
pub fn get_parameters() -> Parameters {
    let parameter_path = std::env::var("PARAMETER_PATH").unwrap();
    let parameters = fs::read_to_string(parameter_path).unwrap();
    let parameters: ParametersFile = serde_json::from_str(&parameters).unwrap();
    parameters.testconnection
}

/// Sets up logging for tests
pub fn setup_logging() {
    let env_filter = EnvFilter::builder()
        .with_default_directive(Level::INFO.into())
        .from_env()
        .unwrap();
    let _ = tracing_subscriber::fmt::fmt()
        .with_env_filter(env_filter)
        .try_init();
}
