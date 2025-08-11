use std::collections::HashMap;

use super::settings::Setting;

pub struct Connection {
    pub settings: HashMap<String, Setting>,
    pub session_token: Option<String>,
}

impl Default for Connection {
    fn default() -> Self {
        Self::new()
    }
}

impl Connection {
    pub fn new() -> Self {
        Connection {
            settings: HashMap::new(),
            session_token: None,
        }
    }
}
