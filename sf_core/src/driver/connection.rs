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

    pub fn build_tls_config_from_settings(&self) -> crate::tls::TlsConfig {
        let mut cfg = crate::tls::TlsConfig::default();
        if let Some(crate::config::settings::Setting::String(path)) =
            self.settings.get("custom_root_store_path")
        {
            cfg.custom_root_store_path = Some(std::path::PathBuf::from(path));
        }
        if let Some(crate::config::settings::Setting::String(v)) =
            self.settings.get("verify_hostname")
        {
            cfg.verify_hostname = v.to_lowercase() == "true";
        }
        if let Some(crate::config::settings::Setting::String(v)) =
            self.settings.get("verify_certificates")
        {
            cfg.verify_certificates = v.to_lowercase() == "true";
        }
        cfg
    }
}
