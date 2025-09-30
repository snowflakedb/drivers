#[cfg(test)]
mod integration_tests {
    use crate::config::rest_parameters::{ClientInfo, LoginMethod, LoginParameters};
    use crate::config::settings::{Setting, Settings};
    use crate::crl::config::{CertRevocationCheckMode, CrlConfig};
    use crate::rest::snowflake;
    use std::collections::HashMap;

    /// Mock settings implementation for testing
    struct MockSettings {
        settings: HashMap<String, Setting>,
    }

    impl MockSettings {
        fn new() -> Self {
            let mut settings = HashMap::new();

            // Add some default settings
            settings.insert(
                "account".to_string(),
                Setting::String("test_account".to_string()),
            );
            settings.insert("user".to_string(), Setting::String("test_user".to_string()));
            settings.insert(
                "password".to_string(),
                Setting::String("test_password".to_string()),
            );
            settings.insert(
                "host".to_string(),
                Setting::String("test.snowflakecomputing.com".to_string()),
            );

            Self { settings }
        }

        fn with_crl_enabled(mut self) -> Self {
            self.settings.insert(
                "crl_check_mode".to_string(),
                Setting::String("ENABLED".to_string()),
            );
            self.settings.insert(
                "crl_enable_disk_caching".to_string(),
                Setting::String("true".to_string()),
            );
            self.settings
                .insert("crl_validity_time".to_string(), Setting::Int(7));
            self
        }
    }

    impl Settings for MockSettings {
        fn get(&self, key: &str) -> Option<Setting> {
            self.settings.get(key).cloned()
        }

        fn set(&mut self, key: &str, value: Setting) {
            self.settings.insert(key.to_string(), value);
        }
    }

    #[test]
    fn test_crl_config_from_settings_disabled() {
        let settings = MockSettings::new();
        let crl_config = CrlConfig::from_settings(&settings).unwrap();

        assert_eq!(crl_config.check_mode, CertRevocationCheckMode::Disabled);
        assert!(crl_config.enable_disk_caching);
        assert!(crl_config.enable_memory_caching);
        assert_eq!(crl_config.validity_time.num_days(), 10);
    }

    #[test]
    fn test_crl_config_from_settings_enabled() {
        let settings = MockSettings::new().with_crl_enabled();
        let crl_config = CrlConfig::from_settings(&settings).unwrap();

        assert_eq!(crl_config.check_mode, CertRevocationCheckMode::Enabled);
        assert!(crl_config.enable_disk_caching);
        assert_eq!(crl_config.validity_time.num_days(), 7);
    }

    #[test]
    fn test_client_info_with_crl_config() {
        let settings = MockSettings::new().with_crl_enabled();
        let client_info = ClientInfo::from_settings(&settings).unwrap();

        assert_eq!(
            client_info.crl_config.check_mode,
            CertRevocationCheckMode::Enabled
        );
        assert_eq!(client_info.application, "PythonConnector");
    }

    #[test]
    fn test_login_parameters_integration() {
        let settings = MockSettings::new().with_crl_enabled();
        let login_params = LoginParameters::from_settings(&settings).unwrap();

        assert_eq!(login_params.account_name, "test_account");
        assert_eq!(
            login_params.client_info.crl_config.check_mode,
            CertRevocationCheckMode::Enabled
        );

        // Verify the login method is correctly parsed
        match login_params.login_method {
            LoginMethod::Password { username, password } => {
                assert_eq!(username, "test_user");
                assert_eq!(password, "test_password");
            }
            _ => panic!("Expected password login method"),
        }
    }

    #[tokio::test]
    async fn test_tls_client_creation_with_different_modes() {
        // Test disabled mode
        let config = CrlConfig {
            check_mode: CertRevocationCheckMode::Disabled,
            ..Default::default()
        };
        let client = crate::tls::create_tls_client_with_config(crate::tls::config::TlsConfig {
            crl_config: config,
            ..Default::default()
        })
        .unwrap();
        assert!(client.get("https://httpbin.org/get").build().is_ok());

        // Test enabled mode
        let config = CrlConfig {
            check_mode: CertRevocationCheckMode::Enabled,
            ..Default::default()
        };
        let client = crate::tls::create_tls_client_with_config(crate::tls::config::TlsConfig {
            crl_config: config,
            ..Default::default()
        })
        .unwrap();
        assert!(client.get("https://httpbin.org/get").build().is_ok());

        // Test advisory mode
        let config = CrlConfig {
            check_mode: CertRevocationCheckMode::Advisory,
            ..Default::default()
        };
        let client = crate::tls::create_tls_client_with_config(crate::tls::config::TlsConfig {
            crl_config: config,
            ..Default::default()
        })
        .unwrap();
        assert!(client.get("https://httpbin.org/get").build().is_ok());
    }

    #[test]
    fn test_user_agent_generation() {
        let settings = MockSettings::new();
        let client_info = ClientInfo::from_settings(&settings).unwrap();
        let user_agent = snowflake::user_agent(&client_info);

        assert!(user_agent.contains("PythonConnector"));
        assert!(user_agent.contains("3.15.0"));
        assert!(user_agent.contains("Darwin"));
    }

    /// Test that demonstrates how CRL settings would be used in a real connection
    #[test]
    fn test_connection_string_parsing_with_crl() {
        let mut settings = MockSettings::new();

        // Simulate parsing connection string parameters
        settings.set("crl_check_mode", Setting::String("ADVISORY".to_string()));
        settings.set(
            "crl_enable_disk_caching",
            Setting::String("false".to_string()),
        );
        settings.set(
            "crl_allow_certificates_without_crl_url",
            Setting::String("true".to_string()),
        );
        settings.set("crl_http_timeout", Setting::Int(45));

        let crl_config = CrlConfig::from_settings(&settings).unwrap();

        assert_eq!(crl_config.check_mode, CertRevocationCheckMode::Advisory);
        assert!(!crl_config.enable_disk_caching);
        assert!(crl_config.allow_certificates_without_crl_url);
        assert_eq!(crl_config.http_timeout.num_seconds(), 45);
    }
}
