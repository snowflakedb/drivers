use serde::{Deserialize, Deserializer, Serialize, Serializer};
use zeroize::Zeroize;

/// A generic wrapper for sensitive values that provides:
/// - Zeroization on drop (via `zeroize`)
/// - Redacted `Debug`/`Display` output (`****`)
/// - `Serialize`/`Deserialize` delegation to the inner type
/// - `Clone`, `Default`
///
/// Use `.reveal()` to access the underlying value.
///
/// # Adding new sensitive types
///
/// ```ignore
/// pub type SensitiveBytes = Sensitive<Vec<u8>>;
/// ```
pub struct Sensitive<T: Zeroize>(T);

pub type SensitiveString = Sensitive<String>;

impl<T: Zeroize> Sensitive<T> {
    pub fn reveal(&self) -> &T {
        &self.0
    }
}

impl<T: Zeroize> Drop for Sensitive<T> {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl<T: Zeroize + Clone> Clone for Sensitive<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: Zeroize> std::fmt::Debug for Sensitive<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("****")
    }
}

impl<T: Zeroize> std::fmt::Display for Sensitive<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("****")
    }
}

impl<T: Zeroize + Default> Default for Sensitive<T> {
    fn default() -> Self {
        Self(T::default())
    }
}

impl<T: Zeroize> From<T> for Sensitive<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl From<&str> for Sensitive<String> {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl<T: Zeroize + Serialize> Serialize for Sensitive<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de, T: Zeroize + Deserialize<'de>> Deserialize<'de> for Sensitive<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        T::deserialize(deserializer).map(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expose_returns_inner_value() {
        let s = SensitiveString::from("secret_123");
        assert_eq!(s.reveal().as_str(), "secret_123");
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
        assert_eq!(s.reveal().as_str(), "");
    }

    #[test]
    fn clone_preserves_value() {
        let a = SensitiveString::from("abc");
        let b = a.clone();
        assert_eq!(b.reveal().as_str(), "abc");
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
        assert_eq!(s.reveal().as_str(), "password_abc");
    }

    #[test]
    fn generic_works_with_vec_u8() {
        let s = Sensitive::<Vec<u8>>::from(vec![1, 2, 3]);
        assert_eq!(s.reveal(), &vec![1, 2, 3]);
        assert_eq!(format!("{s:?}"), "****");
    }
}
