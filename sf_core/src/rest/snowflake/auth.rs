use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// TODO: Delete all unused fields when we are sure they are not needed

// TODO: Currently this is only compatible with Python, should be generalized later
#[derive(Debug, Serialize)]
pub struct AuthRequestClientEnvironment {
    #[serde(rename = "APPLICATION")]
    pub application: String,
    #[serde(rename = "OS")]
    pub os: String,
    #[serde(rename = "OS_VERSION")]
    pub os_version: String,
    #[serde(rename = "OCSP_MODE", skip_serializing_if = "Option::is_none")]
    pub ocsp_mode: Option<String>,
    #[serde(rename = "PYTHON_VERSION", skip_serializing_if = "Option::is_none")]
    pub python_version: Option<String>,
    #[serde(rename = "PYTHON_RUNTIME", skip_serializing_if = "Option::is_none")]
    pub python_runtime: Option<String>,
    #[serde(rename = "PYTHON_COMPILER", skip_serializing_if = "Option::is_none")]
    pub python_compiler: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AuthRequestData {
    #[serde(rename = "CLIENT_APP_ID")]
    pub client_app_id: String,
    #[serde(rename = "CLIENT_APP_VERSION")]
    pub client_app_version: String,
    #[serde(rename = "SVN_REVISION")]
    pub _svn_revision: Option<String>,
    #[serde(rename = "ACCOUNT_NAME")]
    pub account_name: String,
    #[serde(rename = "LOGIN_NAME", skip_serializing_if = "Option::is_none")]
    pub login_name: Option<String>,
    #[serde(rename = "PASSWORD", skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(rename = "RAW_SAML_RESPONSE", skip_serializing_if = "Option::is_none")]
    pub raw_saml_response: Option<String>,
    #[serde(
        rename = "EXT_AUTHN_DUO_METHOD",
        skip_serializing_if = "Option::is_none"
    )]
    pub ext_authn_duo_method: Option<String>,
    #[serde(rename = "PASSCODE", skip_serializing_if = "Option::is_none")]
    pub passcode: Option<String>,
    #[serde(rename = "AUTHENTICATOR", skip_serializing_if = "Option::is_none")]
    pub authenticator: Option<String>,
    #[serde(rename = "SESSION_PARAMETERS", skip_serializing_if = "Option::is_none")]
    pub session_parameters: Option<HashMap<String, serde_json::Value>>,
    #[serde(rename = "CLIENT_ENVIRONMENT")]
    pub client_environment: AuthRequestClientEnvironment,
    #[serde(
        rename = "BROWSER_MODE_REDIRECT_PORT",
        skip_serializing_if = "Option::is_none"
    )]
    pub browser_mode_redirect_port: Option<String>,
    #[serde(rename = "PROOF_KEY", skip_serializing_if = "Option::is_none")]
    pub proof_key: Option<String>,
    #[serde(rename = "TOKEN", skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    #[serde(rename = "OAUTH_TYPE", skip_serializing_if = "Option::is_none")]
    pub oauth_type: Option<String>,
    #[serde(rename = "PROVIDER", skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AuthRequest {
    pub data: AuthRequestData,
}

#[derive(Debug, Deserialize)]
pub struct NameValueParameter {
    #[serde(rename = "name")]
    pub _name: String,
    #[serde(rename = "value")]
    pub _value: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct AuthResponseSessionInfo {
    #[serde(rename = "databaseName")]
    pub _database_name: String,
    #[serde(rename = "schemaName")]
    pub _schema_name: String,
    #[serde(rename = "warehouseName")]
    pub _warehouse_name: String,
    #[serde(rename = "roleName")]
    pub _role_name: String,
}

#[derive(Debug, Deserialize)]
pub struct AuthResponseMain {
    pub token: Option<String>,
    #[serde(rename = "validityInSeconds")]
    pub _validity: Option<u64>,
    #[serde(rename = "masterToken")]
    pub _master_token: Option<String>,
    #[serde(rename = "masterValidityInSeconds")]
    pub _master_validity: u64,
    #[serde(rename = "mfaToken")]
    pub _mfa_token: Option<String>,
    #[serde(rename = "mfaTokenValidityInSeconds")]
    pub _mfa_token_validity: Option<u64>,
    #[serde(rename = "idToken")]
    pub _id_token: Option<String>,
    #[serde(rename = "idTokenValidityInSeconds")]
    pub _id_token_validity: Option<u64>,
    #[serde(rename = "displayUserName")]
    pub _display_user_name: String,
    #[serde(rename = "serverVersion")]
    pub _server_version: String,
    #[serde(rename = "firstLogin")]
    pub _first_login: bool,
    #[serde(rename = "remMeToken")]
    pub _rem_me_token: Option<String>,
    #[serde(rename = "remMeValidityInSeconds")]
    pub _rem_me_validity: u64,
    #[serde(rename = "healthCheckInterval")]
    pub _health_check_interval: u64,
    #[serde(rename = "newClientForUpgrade")]
    pub _new_client_for_upgrade: Option<String>,
    #[serde(rename = "sessionId")]
    pub _session_id: i64,
    #[serde(rename = "parameters")]
    pub _parameters: Option<Vec<NameValueParameter>>,
    #[serde(rename = "sessionInfo")]
    pub _session_info: AuthResponseSessionInfo,
    #[serde(rename = "tokenUrl")]
    pub _token_url: Option<String>,
    #[serde(rename = "ssoUrl")]
    pub _sso_url: Option<String>,
    #[serde(rename = "proofKey")]
    pub _proof_key: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AuthResponse {
    pub data: AuthResponseMain,
    pub message: Option<String>,
    #[serde(rename = "code")]
    pub _code: Option<String>,
    pub success: bool,
}
