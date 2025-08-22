use std::collections::HashMap;

use crate::config::settings::Setting;

pub struct Database {
    pub settings: HashMap<String, Setting>,
}

impl Default for Database {
    fn default() -> Self {
        Self::new()
    }
}

impl Database {
    pub fn new() -> Self {
        Database {
            settings: HashMap::new(),
        }
    }
}
