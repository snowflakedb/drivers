use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use crate::secrets::SecretString;

/// Deserialize seconds as Duration
pub fn deserialize_seconds_as_duration<'de, D>(
    deserializer: D,
) -> Result<Option<Duration>, D::Error>
where
    D: Deserializer<'de>,
{
    let secs: Option<u64> = Option::deserialize(deserializer)?;
    Ok(secs.map(Duration::from_secs))
}

// TODO: Delete all unused fields when we are sure they are not needed

// TODO: Currently this is only compatible with Python, should be generalized later
#[derive(Debug, Serialize, Default, Clone)]
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

#[derive(Debug, Serialize, Default)]
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

/// Safe version of AuthRequestData for logging with sensitive fields redacted
#[derive(Debug, Serialize)]
pub struct SafeAuthRequestData {
    #[serde(rename = "CLIENT_APP_ID")]
    pub client_app_id: String,
    #[serde(rename = "CLIENT_APP_VERSION")]
    pub client_app_version: String,
    #[serde(rename = "ACCOUNT_NAME")]
    pub account_name: String,
    #[serde(rename = "LOGIN_NAME", skip_serializing_if = "Option::is_none")]
    pub login_name: Option<String>,
    #[serde(rename = "PASSWORD", skip_serializing_if = "Option::is_none")]
    pub password: Option<SecretString>,
    #[serde(rename = "RAW_SAML_RESPONSE", skip_serializing_if = "Option::is_none")]
    pub raw_saml_response: Option<SecretString>,
    #[serde(rename = "PASSCODE", skip_serializing_if = "Option::is_none")]
    pub passcode: Option<SecretString>,
    #[serde(rename = "AUTHENTICATOR", skip_serializing_if = "Option::is_none")]
    pub authenticator: Option<String>,
    #[serde(rename = "PROOF_KEY", skip_serializing_if = "Option::is_none")]
    pub proof_key: Option<SecretString>,
    #[serde(rename = "TOKEN", skip_serializing_if = "Option::is_none")]
    pub token: Option<SecretString>,
    #[serde(rename = "CLIENT_ENVIRONMENT")]
    pub client_environment: AuthRequestClientEnvironment,
}

impl AuthRequestData {
    /// Convert to a safe version suitable for logging (sensitive fields redacted)
    pub fn to_safe(&self) -> SafeAuthRequestData {
        SafeAuthRequestData {
            client_app_id: self.client_app_id.clone(),
            client_app_version: self.client_app_version.clone(),
            account_name: self.account_name.clone(),
            login_name: self.login_name.clone(),
            password: self.password.as_ref().map(|_| SecretString::from_str("****")),
            raw_saml_response: self
                .raw_saml_response
                .as_ref()
                .map(|_| SecretString::from_str("****")),
            passcode: self.passcode.as_ref().map(|_| SecretString::from_str("****")),
            authenticator: self.authenticator.clone(),
            proof_key: self.proof_key.as_ref().map(|_| SecretString::from_str("****")),
            token: self.token.as_ref().map(|_| SecretString::from_str("****")),
            client_environment: self.client_environment.clone(),
        }
    }
}

/// Safe version of AuthRequest for logging
#[derive(Debug, Serialize)]
pub struct SafeAuthRequest {
    pub data: SafeAuthRequestData,
}

impl AuthRequest {
    /// Convert to a safe version suitable for logging (sensitive fields redacted)
    pub fn to_safe(&self) -> SafeAuthRequest {
        SafeAuthRequest {
            data: self.data.to_safe(),
        }
    }
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
    pub _database_name: Option<String>,
    #[serde(rename = "schemaName")]
    pub _schema_name: Option<String>,
    #[serde(rename = "warehouseName")]
    pub _warehouse_name: Option<String>,
    #[serde(rename = "roleName")]
    pub _role_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AuthResponseMain {
    /// Session token for authenticating requests
    pub token: Option<String>,
    /// Session token validity
    #[serde(
        rename = "validityInSeconds",
        deserialize_with = "deserialize_seconds_as_duration",
        default
    )]
    pub validity: Option<Duration>,
    /// Master token for refreshing expired session tokens
    #[serde(rename = "masterToken")]
    pub master_token: Option<String>,
    /// Master token validity
    #[serde(
        rename = "masterValidityInSeconds",
        deserialize_with = "deserialize_seconds_as_duration",
        default
    )]
    pub master_validity: Option<Duration>,
    #[serde(rename = "mfaToken")]
    pub _mfa_token: Option<String>,
    #[serde(rename = "mfaTokenValidityInSeconds")]
    pub _mfa_token_validity: Option<u64>,
    #[serde(rename = "idToken")]
    pub _id_token: Option<String>,
    #[serde(rename = "idTokenValidityInSeconds")]
    pub _id_token_validity: Option<u64>,
    #[serde(rename = "displayUserName")]
    pub _display_user_name: Option<String>,
    #[serde(rename = "serverVersion")]
    pub _server_version: Option<String>,
    #[serde(rename = "firstLogin")]
    pub _first_login: Option<bool>,
    #[serde(rename = "remMeToken")]
    pub _rem_me_token: Option<String>,
    #[serde(rename = "remMeValidityInSeconds")]
    pub _rem_me_validity: Option<u64>,
    #[serde(rename = "healthCheckInterval")]
    pub _health_check_interval: Option<u64>,
    #[serde(rename = "newClientForUpgrade")]
    pub _new_client_for_upgrade: Option<String>,
    /// Session ID for the current session
    #[serde(rename = "sessionId")]
    pub session_id: Option<i64>,
    #[serde(rename = "parameters")]
    pub _parameters: Option<Vec<NameValueParameter>>,
    #[serde(rename = "sessionInfo")]
    pub _session_info: Option<AuthResponseSessionInfo>,
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
