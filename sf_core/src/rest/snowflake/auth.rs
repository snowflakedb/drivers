use crate::sensitive::SensitiveString;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::time::Duration;

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

#[derive(Debug, Serialize, Default)]
pub struct AuthRequestClientCapabilities {
    #[serde(rename = "SMK_ID_AS_STRING")]
    pub smk_id_as_string: bool,
}

#[derive(Debug, Serialize, Default)]
pub struct AuthRequestClientEnvironment {
    #[serde(rename = "APPLICATION")]
    pub application: String,
    #[serde(rename = "OS")]
    pub os: String,
    #[serde(rename = "OS_VERSION")]
    pub os_version: String,
    #[serde(rename = "OCSP_MODE", skip_serializing_if = "Option::is_none")]
    pub ocsp_mode: Option<String>,
    #[serde(rename = "PLATFORM")]
    pub platforms: Vec<String>,
    #[serde(rename = "RUNTIME_VERSION", skip_serializing_if = "Option::is_none")]
    pub runtime_version: Option<String>,
    #[serde(rename = "RUNTIME_NAME", skip_serializing_if = "Option::is_none")]
    pub runtime_name: Option<String>,
    #[serde(rename = "COMPILER", skip_serializing_if = "Option::is_none")]
    pub compiler: Option<String>,
    #[serde(rename = "OS_DETAILS", skip_serializing_if = "Option::is_none")]
    pub os_details: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Default)]
pub struct AuthRequestData {
    #[serde(rename = "CLIENT_APP_ID")]
    pub client_app_id: String,
    #[serde(rename = "CLIENT_APP_VERSION")]
    pub client_app_version: String,
    #[serde(rename = "CLIENT_APP_VERSION_FULL")]
    pub client_app_version_full: String,
    #[serde(rename = "SVN_REVISION")]
    pub _svn_revision: Option<String>,
    #[serde(rename = "ACCOUNT_NAME")]
    pub account_name: String,
    #[serde(rename = "LOGIN_NAME", skip_serializing_if = "Option::is_none")]
    pub login_name: Option<String>,
    #[serde(rename = "PASSWORD", skip_serializing_if = "Option::is_none")]
    pub password: Option<SensitiveString>,
    #[serde(rename = "RAW_SAML_RESPONSE", skip_serializing_if = "Option::is_none")]
    pub raw_saml_response: Option<SensitiveString>,
    #[serde(
        rename = "EXT_AUTHN_DUO_METHOD",
        skip_serializing_if = "Option::is_none"
    )]
    pub ext_authn_duo_method: Option<String>,
    #[serde(
        rename = "CLIENT_REQUEST_MFA_TOKEN",
        skip_serializing_if = "Option::is_none"
    )]
    pub client_request_mfa_token: Option<bool>,
    #[serde(rename = "PASSCODE", skip_serializing_if = "Option::is_none")]
    pub passcode: Option<SensitiveString>,
    #[serde(rename = "AUTHENTICATOR", skip_serializing_if = "Option::is_none")]
    pub authenticator: Option<String>,
    #[serde(rename = "SESSION_PARAMETERS", skip_serializing_if = "Option::is_none")]
    pub session_parameters: Option<HashMap<String, serde_json::Value>>,
    #[serde(rename = "CLIENT_CAPABILITIES")]
    pub client_capabilities: AuthRequestClientCapabilities,
    #[serde(rename = "CLIENT_ENVIRONMENT")]
    pub client_environment: AuthRequestClientEnvironment,
    #[serde(
        rename = "BROWSER_MODE_REDIRECT_PORT",
        skip_serializing_if = "Option::is_none"
    )]
    pub browser_mode_redirect_port: Option<String>,
    #[serde(rename = "PROOF_KEY", skip_serializing_if = "Option::is_none")]
    pub proof_key: Option<SensitiveString>,
    #[serde(rename = "TOKEN", skip_serializing_if = "Option::is_none")]
    pub token: Option<SensitiveString>,
    #[serde(rename = "OAUTH_TYPE", skip_serializing_if = "Option::is_none")]
    pub oauth_type: Option<String>,
    #[serde(rename = "PROVIDER", skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(rename = "SPCS_TOKEN", skip_serializing_if = "Option::is_none")]
    pub spcs_token: Option<String>,
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
    pub database_name: Option<String>,
    #[serde(rename = "schemaName")]
    pub schema_name: Option<String>,
    #[serde(rename = "warehouseName")]
    pub warehouse_name: Option<String>,
    #[serde(rename = "roleName")]
    pub role_name: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct AuthResponseMain {
    /// Session token for authenticating requests
    pub token: Option<SensitiveString>,
    /// Session token validity
    #[serde(
        rename = "validityInSeconds",
        deserialize_with = "deserialize_seconds_as_duration",
        default
    )]
    pub validity: Option<Duration>,
    /// Master token for refreshing expired session tokens
    #[serde(rename = "masterToken")]
    pub master_token: Option<SensitiveString>,
    /// Master token validity
    #[serde(
        rename = "masterValidityInSeconds",
        deserialize_with = "deserialize_seconds_as_duration",
        default
    )]
    pub master_validity: Option<Duration>,
    #[serde(rename = "mfaToken")]
    pub mfa_token: Option<SensitiveString>,
    #[serde(rename = "idToken")]
    pub _id_token: Option<SensitiveString>,
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
    pub session_info: Option<AuthResponseSessionInfo>,
    #[serde(rename = "tokenUrl")]
    pub _token_url: Option<String>,
    #[serde(rename = "ssoUrl")]
    pub _sso_url: Option<String>,
    #[serde(rename = "proofKey")]
    pub _proof_key: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AuthResponse {
    #[serde(default)]
    pub data: AuthResponseMain,
    pub message: Option<String>,
    #[serde(rename = "code")]
    pub _code: Option<String>,
    pub success: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_environment_serializes_with_expected_keys() {
        let env = AuthRequestClientEnvironment {
            application: "JDBC".to_string(),
            os: "Linux".to_string(),
            os_version: "Linux-5.10-aarch64-64bit".to_string(),
            ocsp_mode: Some("FAIL_OPEN".to_string()),
            runtime_version: Some("21.0.1".to_string()),
            runtime_name: Some("OpenJDK".to_string()),
            compiler: Some("javac 21.0.1".to_string()),
            ..Default::default()
        };
        let json = serde_json::to_value(&env).unwrap();
        assert_eq!(json["APPLICATION"], "JDBC");
        assert_eq!(json["OS"], "Linux");
        assert_eq!(json["OS_VERSION"], "Linux-5.10-aarch64-64bit");
        assert_eq!(json["OCSP_MODE"], "FAIL_OPEN");
        assert_eq!(json["RUNTIME_VERSION"], "21.0.1");
        assert_eq!(json["RUNTIME_NAME"], "OpenJDK");
        assert_eq!(json["COMPILER"], "javac 21.0.1");
    }

    #[test]
    fn test_client_environment_omits_none_runtime_fields() {
        let env = AuthRequestClientEnvironment {
            application: "ODBC".to_string(),
            os: "Windows".to_string(),
            os_version: "10.0".to_string(),
            ..Default::default()
        };
        let json = serde_json::to_value(&env).unwrap();
        assert!(
            json.get("RUNTIME_VERSION").is_none(),
            "None fields should be skipped"
        );
        assert!(
            json.get("RUNTIME_NAME").is_none(),
            "None fields should be skipped"
        );
        assert!(
            json.get("COMPILER").is_none(),
            "None fields should be skipped"
        );
        assert!(
            json.get("OCSP_MODE").is_none(),
            "None OCSP_MODE should be skipped"
        );
    }
}
