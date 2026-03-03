//! Sensitive data wrapper that is automatically zeroized when dropped.
//!
//! Provides a single wrapper type for all sensitive data (passwords, tokens,
//! private keys, etc.) ensuring memory is securely zeroed when no longer needed.
//!
//! # Security Properties
//!
//! - Memory is overwritten with zeros on drop (ASVS 8.3.6)
//! - Debug/Display implementations hide the actual value
//! - Explicit `.expose()` required to access the underlying data

use secrecy::{ExposeSecret, SecretString};

/// A string value that is automatically zeroized when dropped.
///
/// Use `.expose()` to access the underlying string value.
/// The caller is responsible for not logging or persisting the exposed value.
#[derive(Clone)]
pub struct SensitiveString(SecretString);

impl SensitiveString {
    pub fn new(value: impl Into<String>) -> Self {
        Self(SecretString::from(value.into()))
    }

    pub fn expose(&self) -> &str {
        self.0.expose_secret()
    }

    pub fn same_as(&self, other: &SensitiveString) -> bool {
        self.0.expose_secret() == other.0.expose_secret()
    }

    pub fn is_empty(&self) -> bool {
        self.0.expose_secret().is_empty()
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
    fn test_debug_does_not_expose_value() {
        let secret = SensitiveString::new("super_secret_123");
        let debug_output = format!("{:?}", secret);
        assert!(!debug_output.contains("super_secret"));
        assert!(debug_output.contains("***"));
    }

    #[test]
    fn test_display_does_not_expose_value() {
        let secret = SensitiveString::new("super_secret_123");
        let display_output = format!("{}", secret);
        assert!(!display_output.contains("super_secret"));
        assert!(display_output.contains("***"));
    }

    #[test]
    fn test_expose_returns_value() {
        let secret = SensitiveString::new("super_secret_123");
        assert_eq!(secret.expose(), "super_secret_123");
    }

    #[test]
    fn test_same_as() {
        let a = SensitiveString::new("token_abc");
        let b = SensitiveString::new("token_abc");
        let c = SensitiveString::new("token_xyz");
        assert!(a.same_as(&b));
        assert!(!a.same_as(&c));
    }

    #[test]
    fn test_is_empty() {
        let empty = SensitiveString::new("");
        let non_empty = SensitiveString::new("value");
        assert!(empty.is_empty());
        assert!(!non_empty.is_empty());
    }
}
