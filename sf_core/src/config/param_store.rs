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
    /// Checks `Setting::Bool` first, then falls back to `Setting::String` with
    /// `"true"` / `"1"` / `"false"` / `"0"` parsing for backward compatibility
    /// with TOML-loaded values and ODBC connection-string encodings. Non-zero
    /// `Setting::Int` values are also accepted. Unrecognised strings return
    /// `None` so the caller falls through to its default rather than silently
    /// degrading to `false`.
    pub fn get_bool(&self, key: ParamKey) -> Option<bool> {
        match self.get(key)? {
            Setting::Bool(b) => Some(*b),
            Setting::String(s) => match s.to_lowercase().as_str() {
                "true" | "1" => Some(true),
                "false" | "0" => Some(false),
                _ => None,
            },
            Setting::Int(i) => Some(*i != 0),
            _ => None,
        }
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
            if let Some(f) = param.default {
                store.insert(param.canonical_name.to_owned(), f());
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
