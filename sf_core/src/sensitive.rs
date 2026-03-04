use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A wrapper around `secrecy::SecretString` that provides:
/// - Zeroization on drop (via `secrecy`)
/// - Redacted `Debug`/`Display` output
/// - `Serialize`/`Deserialize` (secrecy 0.10's `SecretString` can't impl these due to `str: !Sized`)
/// - `Default`, `Clone`
///
/// Use `.expose()` to access the underlying `&str`.
#[derive(Clone)]
pub struct SensitiveString(SecretString);

impl SensitiveString {
    pub fn expose(&self) -> &str {
        self.0.expose_secret()
    }
}

impl std::fmt::Debug for SensitiveString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("****")
    }
}

impl std::fmt::Display for SensitiveString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("****")
    }
}

impl Default for SensitiveString {
    fn default() -> Self {
        Self(SecretString::from(""))
    }
}

impl From<String> for SensitiveString {
    fn from(s: String) -> Self {
        Self(SecretString::from(s))
    }
}

impl From<&str> for SensitiveString {
    fn from(s: &str) -> Self {
        Self(SecretString::from(s))
    }
}

impl Serialize for SensitiveString {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.expose_secret().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SensitiveString {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(Self::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expose_returns_inner_value() {
        let s = SensitiveString::from("secret_123");
        assert_eq!(s.expose(), "secret_123");
    }

    #[test]
    fn debug_is_redacted() {
        let s = SensitiveString::from("secret_123");
        assert_eq!(format!("{s:?}"), "****");
    }

    #[test]
    fn display_is_redacted() {
        let s = SensitiveString::from("secret_123");
        assert_eq!(format!("{s}"), "****");
    }

    #[test]
    fn default_is_empty() {
        let s = SensitiveString::default();
        assert_eq!(s.expose(), "");
    }

    #[test]
    fn clone_preserves_value() {
        let a = SensitiveString::from("abc");
        let b = a.clone();
        assert_eq!(b.expose(), "abc");
    }

    #[test]
    fn serialize_exposes_value() {
        let s = SensitiveString::from("token_xyz");
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(json, r#""token_xyz""#);
    }

    #[test]
    fn deserialize_wraps_value() {
        let s: SensitiveString = serde_json::from_str(r#""password_abc""#).unwrap();
        assert_eq!(s.expose(), "password_abc");
    }
}
