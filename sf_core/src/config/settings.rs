use std::collections::HashMap;

#[derive(Clone, Debug)]
pub enum Setting {
    String(String),
    Bytes(Vec<u8>),
    Int(i64),
    Double(f64),
}

impl Setting {
    fn as_string(&self) -> Option<&String> {
        if let Setting::String(value) = self {
            Some(value)
        } else {
            None
        }
    }

    fn as_int(&self) -> Option<&i64> {
        if let Setting::Int(value) = self {
            Some(value)
        } else {
            None
        }
    }

    #[allow(dead_code)]
    fn as_double(&self) -> Option<&f64> {
        if let Setting::Double(value) = self {
            Some(value)
        } else {
            None
        }
    }

    #[allow(dead_code)]
    fn as_bytes(&self) -> Option<&Vec<u8>> {
        if let Setting::Bytes(value) = self {
            Some(value)
        } else {
            None
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
    #[allow(dead_code)]
    fn get_double(&self, key: &str) -> Option<f64> {
        let setting = self.get(key)?;
        setting.as_double().cloned()
    }
    #[allow(dead_code)]
    fn get_bytes(&self, key: &str) -> Option<Vec<u8>> {
        let setting = self.get(key)?;
        setting.as_bytes().cloned()
    }
    #[allow(dead_code)]
    fn set(&mut self, key: &str, value: Setting);
    #[allow(dead_code)]
    fn set_string(&mut self, key: &str, value: String) {
        self.set(key, Setting::String(value));
    }
    #[allow(dead_code)]
    fn set_int(&mut self, key: &str, value: i64) {
        self.set(key, Setting::Int(value));
    }
    #[allow(dead_code)]
    fn set_double(&mut self, key: &str, value: f64) {
        self.set(key, Setting::Double(value));
    }
    #[allow(dead_code)]
    fn set_bytes(&mut self, key: &str, value: Vec<u8>) {
        self.set(key, Setting::Bytes(value));
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
