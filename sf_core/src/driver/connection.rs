use std::collections::HashMap;

use crate::config::settings::Setting;

pub struct Connection {
    pub settings: HashMap<String, Setting>,
    pub session_token: Option<String>,
    pub http_client: Option<reqwest::Client>,
}

impl Connection {
    pub fn new(_db: &std::sync::Mutex<crate::driver::database::Database>) -> Self {
        Self {
            settings: Default::default(),
            session_token: None,
            http_client: None,
        }
    }

    pub fn set_http_client(&mut self, client: reqwest::Client) {
        self.http_client = Some(client);
    }
}
