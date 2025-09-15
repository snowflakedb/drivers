use std::collections::HashMap;

use crate::config::settings::Setting;

#[derive(Default)]
pub struct Connection {
    pub settings: HashMap<String, Setting>,
    pub session_token: Option<String>,
    pub http_client: Option<reqwest::Client>,
}

impl Connection {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_http_client(&mut self, client: reqwest::Client) {
        self.http_client = Some(client);
    }
}
