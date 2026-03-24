use std::collections::HashMap;

use super::param_registry::ParamKey;
use super::settings::{Setting, Settings};

#[derive(Debug)]
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

    pub fn insert(&mut self, key: String, value: Setting) {
        self.inner.insert(key, value);
    }

    pub(crate) fn insert_if_absent(&mut self, key: String, value: Setting) {
        self.inner.entry(key).or_insert(value);
    }

    /// Copy all entries from `other` into `self`, overwriting any existing
    /// values for the same key. This is consumed by later stack changes when
    /// config layering moves into the resolver.
    #[allow(dead_code)]
    pub(crate) fn extend_from(&mut self, other: &ParamStore) {
        for (k, v) in &other.inner {
            self.inner.insert(k.clone(), v.clone());
        }
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
