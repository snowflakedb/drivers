//! Secrets masking utilities to prevent credential leakage in logs
//!
//! This module provides types and utilities for safely handling sensitive data
//! like passwords, tokens, and API keys to prevent accidental exposure in logs,
//! debug output, or error messages.

use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize, Serializer};
use std::fmt;

lazy_static! {
    /// Regex pattern for PASSWORD = 'value'
    static ref RE_PASSWORD: Regex = Regex::new(r"(?i)PASSWORD\s*=\s*'[^']*'").unwrap();

    /// Regex pattern for IDENTIFIED BY 'value'
    static ref RE_IDENTIFIED: Regex = Regex::new(r"(?i)IDENTIFIED\s+BY\s+'[^']*'").unwrap();

    /// Regex pattern for TOKEN = 'value'
    static ref RE_TOKEN: Regex = Regex::new(r"(?i)TOKEN\s*=\s*'[^']*'").unwrap();
}

/// Redacted placeholder shown in logs instead of actual secrets
const REDACTED: &str = "****";

/// A wrapper type for sensitive strings that automatically redacts the value
/// when displayed, logged, or serialized.
///
/// # Examples
///
/// ```
/// use sf_core::secrets::SecretString;
///
/// let password = SecretString::new("my_secret_password".to_string());
/// println!("{}", password); // Prints: "****"
/// assert_eq!(password.expose_secret(), "my_secret_password");
/// ```
#[derive(Clone, PartialEq, Eq)]
pub struct SecretString(String);

impl SecretString {
    /// Create a new SecretString from a String
    pub fn new(secret: String) -> Self {
        Self(secret)
    }

    /// Create a new SecretString from a string slice
    pub fn from_str(secret: &str) -> Self {
        Self(secret.to_string())
    }

    /// Expose the actual secret value (use sparingly and never in logs)
    pub fn expose_secret(&self) -> &str {
        &self.0
    }

    /// Convert to an Option<String>, consuming self
    pub fn into_option(self) -> Option<String> {
        Some(self.0)
    }

    /// Check if the secret is empty
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl From<String> for SecretString {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for SecretString {
    fn from(s: &str) -> Self {
        Self::from_str(s)
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", REDACTED)
    }
}

impl fmt::Display for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", REDACTED)
    }
}

impl Serialize for SecretString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(REDACTED)
    }
}

impl<'de> Deserialize<'de> for SecretString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer).map(SecretString::new)
    }
}

/// Mask a string value for safe logging
///
/// This function determines if a string contains sensitive data based on its
/// field name or content patterns and returns either the redacted value or
/// a partially masked version.
///
/// # Examples
///
/// ```
/// use sf_core::secrets::mask_value;
///
/// assert_eq!(mask_value("password", Some("secret123")), "****");
/// assert_eq!(mask_value("username", Some("alice")), "alice");
/// assert_eq!(mask_value("token", None), "None");
/// ```
pub fn mask_value(field_name: &str, value: Option<&str>) -> String {
    match value {
        None => "None".to_string(),
        Some(val) if is_sensitive_field(field_name) => REDACTED.to_string(),
        Some(val) if looks_like_secret(val) => REDACTED.to_string(),
        Some(val) => val.to_string(),
    }
}

/// Check if a field name indicates sensitive data
fn is_sensitive_field(field_name: &str) -> bool {
    let field_lower = field_name.to_lowercase();
    matches!(
        field_lower.as_str(),
        "password"
            | "token"
            | "secret"
            | "api_key"
            | "apikey"
            | "api-key"
            | "key"
            | "private_key"
            | "privatekey"
            | "private-key"
            | "auth"
            | "authorization"
            | "credential"
            | "credentials"
            | "saml_response"
            | "raw_saml_response"
            | "proof_key"
            | "proofkey"
            | "proof-key"
            | "master_token"
            | "mastertoken"
            | "session_token"
            | "sessiontoken"
            | "access_token"
            | "accesstoken"
            | "refresh_token"
            | "refreshtoken"
            | "bearer"
            | "jwt"
            | "passcode"
            | "passphrase"
            | "oauth"
    )
}

/// Check if a string value looks like a secret based on patterns
fn looks_like_secret(value: &str) -> bool {
    // Empty or very short strings are probably not secrets
    if value.len() < 8 {
        return false;
    }

    // Common token/key patterns
    let value_lower = value.to_lowercase();

    // JWT tokens (three base64 segments separated by dots)
    if value.matches('.').count() == 2 && value.len() > 100 {
        return true;
    }

    // Bearer tokens
    if value_lower.starts_with("bearer ") {
        return true;
    }

    // Common key prefixes
    let secret_prefixes = [
        "sk_", "pk_", "api_", "token_", "key_", "secret_", "aws_", "ghp_", "gho_", "github_pat_",
    ];
    for prefix in &secret_prefixes {
        if value_lower.starts_with(prefix) {
            return true;
        }
    }

    // High entropy strings (likely to be tokens/keys)
    // Simple heuristic: if it's long, has mixed case, numbers, and special chars
    if value.len() > 20 {
        let has_upper = value.chars().any(|c| c.is_uppercase());
        let has_lower = value.chars().any(|c| c.is_lowercase());
        let has_digit = value.chars().any(|c| c.is_numeric());
        let has_special = value
            .chars()
            .any(|c| !c.is_alphanumeric() && c != '-' && c != '_');

        if (has_upper && has_lower && has_digit) || (has_upper && has_lower && has_special) {
            return true;
        }
    }

    false
}

/// Mask a string with partial visibility (shows first and last few characters)
///
/// Useful for logging identifiers where you want some visibility but not full exposure.
///
/// # Examples
///
/// ```
/// use sf_core::secrets::mask_partial;
///
/// assert_eq!(mask_partial("1234567890", 2), "12****90");
/// assert_eq!(mask_partial("short", 2), "sh****");
/// ```
pub fn mask_partial(value: &str, visible_chars: usize) -> String {
    if value.len() <= visible_chars * 2 {
        return REDACTED.to_string();
    }

    let start = &value[..visible_chars];
    let end = &value[value.len() - visible_chars..];
    format!("{}****{}", start, end)
}

/// Redact sensitive query parameters from a SQL query for safe logging
///
/// This function attempts to mask common patterns where secrets might appear
/// in SQL queries, such as CREATE USER statements with passwords.
///
/// # Examples
///
/// ```
/// use sf_core::secrets::redact_query;
///
/// let sql = "CREATE USER alice PASSWORD = 'secret123'";
/// assert!(redact_query(sql).contains("****"));
/// ```
pub fn redact_query(query: &str) -> String {
    // Pattern: PASSWORD = 'value' or PASSWORD='value'
    let query = RE_PASSWORD.replace_all(query, "PASSWORD = '****'");

    // Pattern: IDENTIFIED BY 'value'
    let query = RE_IDENTIFIED.replace_all(&query, "IDENTIFIED BY '****'");

    // Pattern: TOKEN = 'value'
    let query = RE_TOKEN.replace_all(&query, "TOKEN = '****'");

    query.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_string_display() {
        let secret = SecretString::new("my_password".to_string());
        assert_eq!(format!("{}", secret), "****");
        assert_eq!(format!("{:?}", secret), "****");
    }

    #[test]
    fn test_secret_string_expose() {
        let secret = SecretString::new("my_password".to_string());
        assert_eq!(secret.expose_secret(), "my_password");
    }

    #[test]
    fn test_secret_string_serialize() {
        let secret = SecretString::new("my_password".to_string());
        let json = serde_json::to_string(&secret).unwrap();
        assert_eq!(json, r#""****""#);
    }

    #[test]
    fn test_is_sensitive_field() {
        assert!(is_sensitive_field("password"));
        assert!(is_sensitive_field("PASSWORD"));
        assert!(is_sensitive_field("token"));
        assert!(is_sensitive_field("api_key"));
        assert!(is_sensitive_field("private_key"));
        assert!(is_sensitive_field("raw_saml_response"));
        assert!(is_sensitive_field("proof_key"));
        assert!(!is_sensitive_field("username"));
        assert!(!is_sensitive_field("account_name"));
    }

    #[test]
    fn test_looks_like_secret() {
        // JWT token
        assert!(looks_like_secret("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c"));

        // Bearer token
        assert!(looks_like_secret("Bearer abc123def456"));

        // API key with prefix
        assert!(looks_like_secret("sk_live_abc123def456"));
        assert!(looks_like_secret("ghp_abc123def456"));

        // High entropy string
        assert!(looks_like_secret("aB3dE5fG7hJ9kL1mN3pQ5rS"));

        // Not secrets
        assert!(!looks_like_secret("username"));
        assert!(!looks_like_secret("alice"));
        assert!(!looks_like_secret("short"));
    }

    #[test]
    fn test_mask_value() {
        assert_eq!(mask_value("password", Some("secret")), "****");
        assert_eq!(mask_value("username", Some("alice")), "alice");
        assert_eq!(mask_value("token", None), "None");
        assert_eq!(
            mask_value("random", Some("sk_live_abc123")),
            "****"
        );
    }

    #[test]
    fn test_mask_partial() {
        assert_eq!(mask_partial("1234567890", 2), "12****90");
        assert_eq!(mask_partial("abcdefghij", 3), "abc****hij");
        assert_eq!(mask_partial("short", 3), "****");
    }

    #[test]
    fn test_redact_query() {
        let sql = "CREATE USER alice PASSWORD = 'secret123'";
        let redacted = redact_query(sql);
        assert!(redacted.contains("****"));
        assert!(!redacted.contains("secret123"));

        let sql2 = "CREATE USER bob IDENTIFIED BY 'password456'";
        let redacted2 = redact_query(sql2);
        assert!(!redacted2.contains("password456"));

        let sql3 = "ALTER USER charlie SET TOKEN='abc123def456'";
        let redacted3 = redact_query(sql3);
        assert!(!redacted3.contains("abc123def456"));
    }
}
