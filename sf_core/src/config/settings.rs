use std::collections::HashMap;

use sf_params_spec::DefaultValue;

#[derive(Clone, Debug, PartialEq)]
pub enum Setting {
    String(String),
    Bytes(Vec<u8>),
    Int(i64),
    Double(f64),
    Bool(bool),
}

/// Materialize a registry [`DefaultValue`] (the `sf_core`-independent IR from
/// [`sf_params_spec`]) into a runtime [`Setting`]. This is the single boundary
/// where the borrowed, `'static` default data becomes an owned `Setting`.
impl From<DefaultValue> for Setting {
    fn from(value: DefaultValue) -> Self {
        match value {
            DefaultValue::String(s) => Setting::String(s.to_owned()),
            DefaultValue::Bytes(b) => Setting::Bytes(b.to_vec()),
            DefaultValue::Int(i) => Setting::Int(i),
            DefaultValue::Double(d) => Setting::Double(d),
            DefaultValue::Bool(b) => Setting::Bool(b),
        }
    }
}

impl Setting {
    pub(crate) fn as_string(&self) -> Option<&String> {
        if let Setting::String(value) = self {
            Some(value)
        } else {
            None
        }
    }

    pub(crate) fn as_int(&self) -> Option<&i64> {
        if let Setting::Int(value) = self {
            Some(value)
        } else {
            None
        }
    }

    pub(crate) fn as_double(&self) -> Option<&f64> {
        if let Setting::Double(value) = self {
            Some(value)
        } else {
            None
        }
    }

    pub(crate) fn as_bytes(&self) -> Option<&Vec<u8>> {
        if let Setting::Bytes(value) = self {
            Some(value)
        } else {
            None
        }
    }

    fn as_bool(&self) -> Option<&bool> {
        if let Setting::Bool(value) = self {
            Some(value)
        } else {
            None
        }
    }

    /// Shared bool coercion for `ParamStore::get_bool` and
    /// `TlsConfig::from_settings`, so both TLS-config paths agree. Returns
    /// `None` for unrecognised strings/types so callers keep their own default.
    pub(crate) fn coerce_bool(&self) -> Option<bool> {
        match self {
            Setting::Bool(b) => Some(*b),
            Setting::String(s) => match s.to_lowercase().as_str() {
                "true" | "1" | "on" => Some(true),
                "false" | "0" | "off" => Some(false),
                _ => None,
            },
            Setting::Int(i) => Some(*i != 0),
            _ => None,
        }
    }
}

pub trait Settings {
    fn get(&self, key: &str) -> Option<Setting>;
    fn get_string(&self, key: &str) -> Option<String> {
        let setting = self.get(key)?;
        setting.as_string().cloned()
    }
    fn get_int(&self, key: &str) -> Option<i64> {
        let setting = self.get(key)?;
        setting.as_int().cloned()
    }
    /// Get a value as u64, trying integer first, then parsing string.
    fn get_u64(&self, key: &str) -> Option<u64> {
        self.get_int(key)
            .and_then(|v| u64::try_from(v).ok())
            .or_else(|| self.get_string(key).and_then(|s| s.parse::<u64>().ok()))
    }

    fn get_double(&self, key: &str) -> Option<f64> {
        let setting = self.get(key)?;
        setting.as_double().cloned()
    }
    fn get_bytes(&self, key: &str) -> Option<Vec<u8>> {
        let setting = self.get(key)?;
        setting.as_bytes().cloned()
    }
    fn get_bool(&self, key: &str) -> Option<bool> {
        let setting = self.get(key)?;
        setting.as_bool().copied()
    }
    /// Read a bool, coercing string (`"true"`/`"false"`/`"1"`/`"0"`/`"on"`/`"off"`)
    /// and int representations via [`Setting::coerce_bool`], falling back to
    /// `default` when the key is absent or unparseable.
    ///
    /// Unlike [`get_bool`](Settings::get_bool), which only matches a native
    /// `Setting::Bool`, this is the right accessor on `&dyn Settings`/`&str`
    /// paths where the value may arrive as a string or int — matching the
    /// coercion `resolve_options` applies and `ParamStore::get_bool` performs.
    fn get_bool_or(&self, key: &str, default: bool) -> bool {
        self.get(key)
            .and_then(|s| s.coerce_bool())
            .unwrap_or(default)
    }
    fn set(&mut self, key: &str, value: Setting);
    fn set_string(&mut self, key: &str, value: String) {
        self.set(key, Setting::String(value));
    }
    fn set_int(&mut self, key: &str, value: i64) {
        self.set(key, Setting::Int(value));
    }
    fn set_double(&mut self, key: &str, value: f64) {
        self.set(key, Setting::Double(value));
    }
    fn set_bytes(&mut self, key: &str, value: Vec<u8>) {
        self.set(key, Setting::Bytes(value));
    }
    fn set_bool(&mut self, key: &str, value: bool) {
        self.set(key, Setting::Bool(value));
    }
}

impl Settings for HashMap<String, Setting> {
    fn get(&self, key: &str) -> Option<Setting> {
        self.get(key).cloned()
    }

    fn set(&mut self, key: &str, value: Setting) {
        self.insert(key.to_string(), value);
    }
}
