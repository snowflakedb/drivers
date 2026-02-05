//! Sensitive data types that are automatically zeroized when dropped.
//!
//! This module provides wrapper types for sensitive data (passwords, tokens, etc.)
//! that ensure the memory is securely zeroed when the data is no longer needed.
//!
//! # Security Properties
//!
//! - Memory is overwritten with zeros on drop (ASVS 8.3.6)
//! - Debug/Display implementations hide the actual value
//! - Explicit `.expose()` required to access the underlying data

use secrecy::{ExposeSecret, SecretString};

/// A password that is automatically zeroized when dropped.
///
/// Use `.expose()` to access the underlying string value.
#[derive(Clone)]
pub struct SensitivePassword(SecretString);

impl SensitivePassword {
    pub fn new(value: impl Into<String>) -> Self {
        Self(SecretString::from(value.into()))
    }

    /// Expose the underlying password value.
    ///
    /// The caller is responsible for not logging or persisting this value.
    pub fn expose(&self) -> &str {
        self.0.expose_secret()
    }
}

impl std::fmt::Debug for SensitivePassword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SensitivePassword(***)")
    }
}

impl std::fmt::Display for SensitivePassword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("***")
    }
}

/// A token (session token, one-time token, etc.) that is automatically zeroized when dropped.
///
/// Use `.expose()` to access the underlying string value.
#[derive(Clone)]
pub struct SensitiveToken(SecretString);

impl SensitiveToken {
    pub fn new(value: impl Into<String>) -> Self {
        Self(SecretString::from(value.into()))
    }

    /// Expose the underlying token value.
    ///
    /// The caller is responsible for not logging or persisting this value.
    pub fn expose(&self) -> &str {
        self.0.expose_secret()
    }

    /// Check if two tokens have the same underlying value.
    ///
    /// This is useful for detecting if a token has been refreshed.
    pub fn same_as(&self, other: &SensitiveToken) -> bool {
        self.0.expose_secret() == other.0.expose_secret()
    }

    /// Check if the underlying token is empty.
    pub fn is_empty(&self) -> bool {
        self.0.expose_secret().is_empty()
    }
}

impl std::fmt::Debug for SensitiveToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SensitiveToken(***)")
    }
}

impl std::fmt::Display for SensitiveToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("***")
    }
}

/// A private key that is automatically zeroized when dropped.
///
/// Use `.expose()` to access the underlying string value.
#[derive(Clone)]
pub struct SensitivePrivateKey(SecretString);

impl SensitivePrivateKey {
    pub fn new(value: impl Into<String>) -> Self {
        Self(SecretString::from(value.into()))
    }

    /// Expose the underlying private key value.
    ///
    /// The caller is responsible for not logging or persisting this value.
    pub fn expose(&self) -> &str {
        self.0.expose_secret()
    }
}

impl std::fmt::Debug for SensitivePrivateKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SensitivePrivateKey(***)")
    }
}

impl std::fmt::Display for SensitivePrivateKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("***")
    }
}

/// Generic sensitive string for other sensitive data (SAML responses, etc.)
///
/// Use `.expose()` to access the underlying string value.
#[derive(Clone)]
pub struct SensitiveString(SecretString);

impl SensitiveString {
    pub fn new(value: impl Into<String>) -> Self {
        Self(SecretString::from(value.into()))
    }

    /// Expose the underlying string value.
    ///
    /// The caller is responsible for not logging or persisting this value.
    pub fn expose(&self) -> &str {
        self.0.expose_secret()
    }
}

impl std::fmt::Debug for SensitiveString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SensitiveString(***)")
    }
}

impl std::fmt::Display for SensitiveString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("***")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_debug_does_not_expose_value() {
        let password = SensitivePassword::new("super_secret_123");
        let debug_output = format!("{:?}", password);
        assert!(!debug_output.contains("super_secret"));
        assert!(debug_output.contains("***"));
    }

    #[test]
    fn test_password_display_does_not_expose_value() {
        let password = SensitivePassword::new("super_secret_123");
        let display_output = format!("{}", password);
        assert!(!display_output.contains("super_secret"));
        assert!(display_output.contains("***"));
    }

    #[test]
    fn test_password_expose_returns_value() {
        let password = SensitivePassword::new("super_secret_123");
        assert_eq!(password.expose(), "super_secret_123");
    }

    #[test]
    fn test_token_debug_does_not_expose_value() {
        let token = SensitiveToken::new("test_session_token_12345");
        let debug_output = format!("{:?}", token);
        assert!(!debug_output.contains("test_session"));
        assert!(debug_output.contains("***"));
    }

    #[test]
    fn test_token_expose_returns_value() {
        let token = SensitiveToken::new("test_session_token_12345");
        assert_eq!(token.expose(), "test_session_token_12345");
    }
}
