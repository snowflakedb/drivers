#[cfg(test)]
mod integration_tests {
    use crate::config::rest_parameters::ClientInfo;
    use crate::config::rest_parameters::test_fixtures::test_client_info;
    use crate::crl::config::{CertRevocationCheckMode, CrlConfig};
    use crate::rest::snowflake;
    use crate::tls::config::TlsConfig;

    #[test]
    fn test_crl_config_disabled_defaults() {
        let crl_config = CrlConfig::default();

        assert_eq!(crl_config.check_mode, CertRevocationCheckMode::Disabled);
        assert!(crl_config.enable_disk_caching);
        assert!(crl_config.enable_memory_caching);
        assert_eq!(crl_config.validity_time.num_days(), 10);
    }

    #[test]
    fn test_crl_config_enabled() {
        let crl_config = CrlConfig {
            check_mode: CertRevocationCheckMode::Enabled,
            enable_disk_caching: true,
            validity_time: chrono::Duration::days(7),
            ..Default::default()
        };

        assert_eq!(crl_config.check_mode, CertRevocationCheckMode::Enabled);
        assert!(crl_config.enable_disk_caching);
        assert_eq!(crl_config.validity_time.num_days(), 7);
    }

    #[test]
    fn test_client_info_with_crl_config() {
        let crl_config = CrlConfig {
            check_mode: CertRevocationCheckMode::Enabled,
            ..Default::default()
        };
        let client_info = ClientInfo {
            application: "PythonConnector".to_string(),
            crl_config: crl_config.clone(),
            tls_config: TlsConfig {
                crl_config,
                ..Default::default()
            },
            ..test_client_info()
        };

        assert_eq!(
            client_info.crl_config.check_mode,
            CertRevocationCheckMode::Enabled
        );
        assert_eq!(client_info.application, "PythonConnector");
    }

    #[test]
    fn test_connection_config_integration() {
        use crate::config::connection_config::{AuthConfig, ConnectionConfig};
        use crate::config::param_store::ParamStore;
        use crate::config::settings::Setting;

        let mut settings = ParamStore::with_registry_defaults();
        for (k, v) in [
            ("account", Setting::String("test_account".into())),
            ("user", Setting::String("test_user".into())),
            ("password", Setting::String("test_password".into())),
            (
                "host",
                Setting::String("test_account.snowflakecomputing.com".into()),
            ),
            ("crl_check_mode", Setting::String("ENABLED".into())),
        ] {
            settings.insert(k.to_string(), v);
        }

        let config = ConnectionConfig::build(&settings).unwrap();
        assert_eq!(config.server.account, "test_account");
        assert_eq!(
            config.tls.crl_config.check_mode,
            CertRevocationCheckMode::Enabled
        );

        match &config.auth {
            AuthConfig::Password { user, password } => {
                assert_eq!(user, "test_user");
                assert_eq!(password.reveal(), "test_password");
            }
            _ => panic!("Expected password auth"),
        }
    }

    #[tokio::test]
    async fn test_tls_client_creation_with_different_modes() {
        // Test disabled mode
        let config = CrlConfig {
            check_mode: CertRevocationCheckMode::Disabled,
            ..Default::default()
        };
        let client = crate::tls::create_tls_client_with_config(TlsConfig {
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
        let client = crate::tls::create_tls_client_with_config(TlsConfig {
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
        let client = crate::tls::create_tls_client_with_config(TlsConfig {
            crl_config: config,
            ..Default::default()
        })
        .unwrap();
        assert!(client.get("https://httpbin.org/get").build().is_ok());
    }

    #[test]
    fn test_user_agent_generation() {
        let client_info = ClientInfo {
            application: "PythonConnector".to_string(),
            version: "3.15.0".to_string(),
            os: "Darwin".to_string(),
            ..test_client_info()
        };
        let user_agent = snowflake::user_agent(&client_info);

        assert!(user_agent.contains("PythonConnector"));
        assert!(user_agent.contains("3.15.0"));
        assert!(user_agent.contains("Darwin"));
    }

    #[test]
    fn test_crl_config_advisory_mode() {
        let crl_config = CrlConfig {
            check_mode: CertRevocationCheckMode::Advisory,
            enable_disk_caching: false,
            allow_certificates_without_crl_url: true,
            http_timeout: chrono::Duration::seconds(45),
            ..Default::default()
        };

        assert_eq!(crl_config.check_mode, CertRevocationCheckMode::Advisory);
        assert!(!crl_config.enable_disk_caching);
        assert!(crl_config.allow_certificates_without_crl_url);
        assert_eq!(crl_config.http_timeout.num_seconds(), 45);
    }
}
