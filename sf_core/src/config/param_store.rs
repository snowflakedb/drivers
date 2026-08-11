use super::param_registry::ParamKey;
use super::settings::{Setting, Settings};
use crate::sensitive::SensitiveString;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ParamStore {
    inner: HashMap<String, Setting>,
}

impl ParamStore {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&String, &Setting)> {
        self.inner.iter()
    }

    pub fn get(&self, key: ParamKey) -> Option<&Setting> {
        self.inner.get(key.as_str())
    }

    /// Lookup by canonical parameter name (any `&str`), for dynamic keys from wrappers.
    pub fn get_any(&self, canonical_key: impl AsRef<str>) -> Option<&Setting> {
        self.inner.get(canonical_key.as_ref())
    }

    /// Extract a string value for `key`, returning `None` if absent or not a string.
    pub fn get_string(&self, key: ParamKey) -> Option<String> {
        match self.get(key)? {
            Setting::String(s) => Some(s.clone()),
            _ => None,
        }
    }

    /// Extract an integer value for `key`.
    ///
    /// Returns `Some` for `Setting::Int` directly. Also accepts
    /// `Setting::String` by attempting a decimal parse, for backward
    /// compatibility with TOML files and connection strings where numeric
    /// values may arrive as quoted strings (e.g. `port = "443"`).
    /// Returns `None` if the key is absent, the value is a non-numeric
    /// string, or the value is any other non-integer type.
    pub fn get_int(&self, key: ParamKey) -> Option<i64> {
        match self.get(key)? {
            Setting::Int(i) => Some(*i),
            Setting::String(s) => s.parse::<i64>().ok(),
            _ => None,
        }
    }

    /// Extract a floating-point value for `key`.
    ///
    /// Returns `Some` for `Setting::Double` directly. Also accepts
    /// `Setting::Int` (widened to `f64`) and `Setting::String` via a decimal
    /// parse, mirroring [`get_int`](Self::get_int)'s tolerance for numeric
    /// values that arrive as quoted strings from TOML files or connection
    /// strings. Returns `None` if the key is absent, the string is
    /// non-numeric, or the value is any other type.
    pub fn get_double(&self, key: ParamKey) -> Option<f64> {
        match self.get(key)? {
            Setting::Double(d) => Some(*d),
            Setting::Int(i) => Some(*i as f64),
            Setting::String(s) => s.parse::<f64>().ok(),
            _ => None,
        }
    }

    /// Extract a string value for `key` and wrap it in [`SensitiveString`],
    /// returning `None` if absent or not a string. Use for credential fields
    /// that must never appear in debug output.
    pub fn get_sensitive_string(&self, key: ParamKey) -> Option<SensitiveString> {
        match self.get(key)? {
            Setting::String(s) => Some(SensitiveString::from(s.clone())),
            _ => None,
        }
    }

    /// Extract a boolean value for `key`.
    ///
    /// Coercion (native `Bool`, `"true"`/`"1"`/`"on"` / `"false"`/`"0"`/`"off"`
    /// strings, non-zero `Int`) is shared with `TlsConfig::from_settings` via
    /// [`Setting::coerce_bool`]. Unrecognised strings return `None` so the
    /// caller falls through to its default rather than degrading to `false`.
    pub fn get_bool(&self, key: ParamKey) -> Option<bool> {
        self.get(key).and_then(Setting::coerce_bool)
    }

    /// Create a `ParamStore` pre-populated with all registry defaults.
    ///
    /// Use this in tests that call `ConnectionConfig::build()` directly,
    /// bypassing `resolver::resolve`. This ensures every param that has a
    /// registry default behaves as if `resolve` had been called, so the
    /// production code path, which assumes defaults are present, does not
    /// need defensive `.unwrap_or(literal)` fallbacks.
    ///
    /// **Do not** use this for a live connection's merged seed + file layers as if it were
    /// the only store:
    /// pre-populating defaults there would override TOML file settings in
    /// `resolver::resolve_with_paths` because explicit settings are applied
    /// last (Layer 1) and would overwrite file settings (Layer 3/2).
    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn with_registry_defaults() -> Self {
        let mut store = Self::new();
        for param in super::param_registry::registry().all_params() {
            if let Some(default) = param.default {
                store.insert(param.canonical_name.to_owned(), default.into());
            }
        }
        store
    }

    /// Insert or overwrite a setting by its canonical string key.
    pub fn insert(&mut self, key: String, value: Setting) {
        self.inner.insert(key, value);
    }

    /// Copy all entries from `other` into `self`, overwriting any existing
    /// values for the same key. Used by `resolver::resolve_with_paths` to
    /// apply each successive config layer onto the merged result.
    pub(crate) fn extend_from(&mut self, other: &ParamStore) {
        for (k, v) in &other.inner {
            self.inner.insert(k.clone(), v.clone());
        }
    }

    /// Iterate over all canonical key names. Used by `validate_settings` to
    /// check for unknown parameters.
    pub(crate) fn keys(&self) -> impl Iterator<Item = &String> {
        self.inner.keys()
    }
}

impl Default for ParamStore {
    fn default() -> Self {
        Self::new()
    }
}

impl Settings for ParamStore {
    fn get(&self, key: &str) -> Option<Setting> {
        self.inner.get(key).cloned()
    }

    fn set(&mut self, key: &str, value: Setting) {
        self.inner.insert(key.to_string(), value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::param_registry::param_names;

    const KEY: ParamKey = param_names::RETRY_BACKOFF_FACTOR;

    fn store_with(value: Setting) -> ParamStore {
        let mut s = ParamStore::new();
        s.insert(KEY.as_str().to_string(), value);
        s
    }

    #[test]
    fn get_double_coerces_numeric_forms_and_rejects_others() {
        // Native double, widened int, and decimal-string forms all succeed.
        assert_eq!(store_with(Setting::Double(1.5)).get_double(KEY), Some(1.5));
        assert_eq!(store_with(Setting::Int(3)).get_double(KEY), Some(3.0));
        assert_eq!(
            store_with(Setting::String("2.25".to_string())).get_double(KEY),
            Some(2.25)
        );
        // Non-numeric string, wrong type, and absent key return None.
        assert_eq!(
            store_with(Setting::String("abc".to_string())).get_double(KEY),
            None
        );
        assert_eq!(store_with(Setting::Bool(true)).get_double(KEY), None);
        assert_eq!(ParamStore::new().get_double(KEY), None);
    }
}
