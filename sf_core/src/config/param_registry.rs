use std::collections::HashMap;

use once_cell::sync::Lazy;

use crate::config::settings::Setting;

/// Defines a single supported configuration parameter.
pub struct ParamDef {
    /// The canonical key name used internally (e.g. `"host"`).
    pub canonical_name: &'static str,

    /// Alternative names accepted from wrappers (case-insensitive lookup).
    /// e.g. `&["SERVER", "HOST"]` all resolve to `"host"`.
    pub aliases: &'static [&'static str],

    /// Expected value type.
    pub value_type: ValueType,

    /// When this parameter is required.
    pub required: Required,

    /// Default value factory, if any.
    pub default: Option<fn() -> Setting>,

    /// Whether the value contains secrets (for log redaction).
    pub sensitive: bool,

    /// Logical grouping (for documentation/validation).
    pub scope: ParamScope,

    /// Human-readable description.
    pub description: &'static str,

    /// If deprecated, the canonical name of the replacement parameter.
    pub deprecated_by: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    String,
    Int,
    #[allow(dead_code)]
    Double,
    #[allow(dead_code)]
    Bytes,
    Bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Required {
    /// Always required (e.g. `account`).
    Always,
    /// Required only when the authenticator matches (e.g. `password` for
    /// `SNOWFLAKE_PASSWORD`).
    WhenAuthMethod(&'static str),
    /// Never required.
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamScope {
    Server,
    Auth,
    Session,
    Tls,
    Crl,
    Client,
}

static PARAM_DEFS: &[ParamDef] = &[
    // ── Server ──────────────────────────────────────────────────────────
    ParamDef {
        canonical_name: "account",
        aliases: &["ACCOUNT"],
        value_type: ValueType::String,
        required: Required::Always,
        default: None,
        sensitive: false,
        scope: ParamScope::Server,
        description: "Snowflake account identifier",
        deprecated_by: None,
    },
    ParamDef {
        canonical_name: "host",
        aliases: &["SERVER", "HOST"],
        value_type: ValueType::String,
        required: Required::Never,
        default: None,
        sensitive: false,
        scope: ParamScope::Server,
        description: "Snowflake server hostname",
        deprecated_by: None,
    },
    ParamDef {
        canonical_name: "port",
        aliases: &["PORT"],
        value_type: ValueType::Int,
        required: Required::Never,
        default: None,
        sensitive: false,
        scope: ParamScope::Server,
        description: "Server port number",
        deprecated_by: None,
    },
    ParamDef {
        canonical_name: "protocol",
        aliases: &["PROTOCOL"],
        value_type: ValueType::String,
        required: Required::Never,
        default: Some(|| Setting::String("https".to_string())),
        sensitive: false,
        scope: ParamScope::Server,
        description: "Connection protocol (http or https)",
        deprecated_by: None,
    },
    ParamDef {
        canonical_name: "server_url",
        aliases: &[],
        value_type: ValueType::String,
        required: Required::Never,
        default: None,
        sensitive: false,
        scope: ParamScope::Server,
        description: "Full server URL (alternative to host/port/protocol)",
        deprecated_by: None,
    },
    // ── Auth ────────────────────────────────────────────────────────────
    ParamDef {
        canonical_name: "user",
        aliases: &["UID"],
        value_type: ValueType::String,
        required: Required::Always,
        default: None,
        sensitive: false,
        scope: ParamScope::Auth,
        description: "Login username",
        deprecated_by: None,
    },
    ParamDef {
        canonical_name: "password",
        aliases: &["PWD"],
        value_type: ValueType::String,
        required: Required::WhenAuthMethod("SNOWFLAKE_PASSWORD"),
        default: None,
        sensitive: true,
        scope: ParamScope::Auth,
        description: "Login password",
        deprecated_by: None,
    },
    ParamDef {
        canonical_name: "authenticator",
        aliases: &["AUTHENTICATOR"],
        value_type: ValueType::String,
        required: Required::Never,
        default: None,
        sensitive: false,
        scope: ParamScope::Auth,
        description: "Authentication method (SNOWFLAKE_PASSWORD, SNOWFLAKE_JWT, PROGRAMMATIC_ACCESS_TOKEN)",
        deprecated_by: None,
    },
    ParamDef {
        canonical_name: "private_key",
        aliases: &["PRIV_KEY_BASE64"],
        value_type: ValueType::String,
        required: Required::WhenAuthMethod("SNOWFLAKE_JWT"),
        default: None,
        sensitive: true,
        scope: ParamScope::Auth,
        description: "Private key for key-pair authentication (base64-encoded or PEM)",
        deprecated_by: None,
    },
    ParamDef {
        canonical_name: "private_key_file",
        aliases: &["PRIV_KEY_FILE"],
        value_type: ValueType::String,
        required: Required::Never,
        default: None,
        sensitive: false,
        scope: ParamScope::Auth,
        description: "Path to private key file for key-pair authentication",
        deprecated_by: None,
    },
    ParamDef {
        canonical_name: "private_key_password",
        aliases: &["PRIV_KEY_FILE_PWD", "PRIV_KEY_PWD"],
        value_type: ValueType::String,
        required: Required::Never,
        default: None,
        sensitive: true,
        scope: ParamScope::Auth,
        description: "Passphrase for encrypted private key",
        deprecated_by: None,
    },
    ParamDef {
        canonical_name: "token",
        aliases: &["TOKEN"],
        value_type: ValueType::String,
        required: Required::WhenAuthMethod("PROGRAMMATIC_ACCESS_TOKEN"),
        default: None,
        sensitive: true,
        scope: ParamScope::Auth,
        description: "Programmatic access token",
        deprecated_by: None,
    },
    // ── Session ─────────────────────────────────────────────────────────
    ParamDef {
        canonical_name: "database",
        aliases: &["DATABASE"],
        value_type: ValueType::String,
        required: Required::Never,
        default: None,
        sensitive: false,
        scope: ParamScope::Session,
        description: "Default database to use",
        deprecated_by: None,
    },
    ParamDef {
        canonical_name: "schema",
        aliases: &["SCHEMA"],
        value_type: ValueType::String,
        required: Required::Never,
        default: None,
        sensitive: false,
        scope: ParamScope::Session,
        description: "Default schema to use",
        deprecated_by: None,
    },
    ParamDef {
        canonical_name: "warehouse",
        aliases: &["WAREHOUSE"],
        value_type: ValueType::String,
        required: Required::Never,
        default: None,
        sensitive: false,
        scope: ParamScope::Session,
        description: "Default warehouse to use",
        deprecated_by: None,
    },
    ParamDef {
        canonical_name: "role",
        aliases: &["ROLE"],
        value_type: ValueType::String,
        required: Required::Never,
        default: None,
        sensitive: false,
        scope: ParamScope::Session,
        description: "Default role to use",
        deprecated_by: None,
    },
    // ── TLS ─────────────────────────────────────────────────────────────
    ParamDef {
        canonical_name: "custom_root_store_path",
        aliases: &["TLS_CUSTOM_ROOT_STORE_PATH"],
        value_type: ValueType::String,
        required: Required::Never,
        default: None,
        sensitive: false,
        scope: ParamScope::Tls,
        description: "Path to custom root certificate store",
        deprecated_by: None,
    },
    ParamDef {
        canonical_name: "verify_hostname",
        aliases: &["TLS_VERIFY_HOSTNAME"],
        value_type: ValueType::Bool,
        required: Required::Never,
        default: Some(|| Setting::Bool(true)),
        sensitive: false,
        scope: ParamScope::Tls,
        description: "Whether to verify the server hostname in TLS",
        deprecated_by: None,
    },
    ParamDef {
        canonical_name: "verify_certificates",
        aliases: &["TLS_VERIFY_CERTIFICATES"],
        value_type: ValueType::Bool,
        required: Required::Never,
        default: Some(|| Setting::Bool(true)),
        sensitive: false,
        scope: ParamScope::Tls,
        description: "Whether to verify TLS certificates",
        deprecated_by: None,
    },
    // ── CRL ─────────────────────────────────────────────────────────────
    ParamDef {
        canonical_name: "crl_check_mode",
        aliases: &["CRL_MODE", "CRL_ENABLED"],
        value_type: ValueType::String,
        required: Required::Never,
        default: Some(|| Setting::String("DISABLED".to_string())),
        sensitive: false,
        scope: ParamScope::Crl,
        description: "Certificate revocation check mode (DISABLED, ENABLED, ADVISORY)",
        deprecated_by: None,
    },
    ParamDef {
        canonical_name: "crl_enable_disk_caching",
        aliases: &[],
        value_type: ValueType::Bool,
        required: Required::Never,
        default: Some(|| Setting::Bool(true)),
        sensitive: false,
        scope: ParamScope::Crl,
        description: "Enable disk caching for CRL responses",
        deprecated_by: None,
    },
    ParamDef {
        canonical_name: "crl_enable_memory_caching",
        aliases: &[],
        value_type: ValueType::Bool,
        required: Required::Never,
        default: Some(|| Setting::Bool(true)),
        sensitive: false,
        scope: ParamScope::Crl,
        description: "Enable in-memory caching for CRL responses",
        deprecated_by: None,
    },
    ParamDef {
        canonical_name: "crl_cache_dir",
        aliases: &[],
        value_type: ValueType::String,
        required: Required::Never,
        default: None,
        sensitive: false,
        scope: ParamScope::Crl,
        description: "Directory for CRL cache files",
        deprecated_by: None,
    },
    ParamDef {
        canonical_name: "crl_validity_time",
        aliases: &[],
        value_type: ValueType::Int,
        required: Required::Never,
        default: Some(|| Setting::Int(10)),
        sensitive: false,
        scope: ParamScope::Crl,
        description: "CRL cache validity time in days",
        deprecated_by: None,
    },
    ParamDef {
        canonical_name: "crl_allow_certificates_without_crl_url",
        aliases: &[],
        value_type: ValueType::Bool,
        required: Required::Never,
        default: Some(|| Setting::Bool(false)),
        sensitive: false,
        scope: ParamScope::Crl,
        description: "Allow certificates that do not include a CRL distribution URL",
        deprecated_by: None,
    },
    ParamDef {
        canonical_name: "crl_http_timeout",
        aliases: &[],
        value_type: ValueType::Int,
        required: Required::Never,
        default: Some(|| Setting::Int(30)),
        sensitive: false,
        scope: ParamScope::Crl,
        description: "HTTP timeout in seconds for CRL endpoint requests",
        deprecated_by: None,
    },
    ParamDef {
        canonical_name: "crl_connection_timeout",
        aliases: &[],
        value_type: ValueType::Int,
        required: Required::Never,
        default: Some(|| Setting::Int(10)),
        sensitive: false,
        scope: ParamScope::Crl,
        description: "Connection timeout in seconds for CRL endpoints",
        deprecated_by: None,
    },
    // ── Client ──────────────────────────────────────────────────────────
    ParamDef {
        canonical_name: "connection_name",
        aliases: &[],
        value_type: ValueType::String,
        required: Required::Never,
        default: None,
        sensitive: false,
        scope: ParamScope::Client,
        description: "Named connection to load from TOML configuration files",
        deprecated_by: None,
    },
];

/// The registry singleton. Built once at startup, immutable thereafter.
pub struct ParamRegistry {
    params: &'static [ParamDef],
    /// Case-insensitive map: lowercased alias/canonical name → index into `params`.
    alias_index: HashMap<String, usize>,
}

impl ParamRegistry {
    fn new(params: &'static [ParamDef]) -> Self {
        let mut alias_index = HashMap::new();
        for (i, param) in params.iter().enumerate() {
            alias_index.insert(param.canonical_name.to_ascii_lowercase(), i);
            for alias in param.aliases {
                alias_index.insert(alias.to_ascii_lowercase(), i);
            }
        }
        Self {
            params,
            alias_index,
        }
    }

    /// Resolve an alias or canonical name to its `ParamDef`.
    /// Lookup is case-insensitive.
    pub fn resolve(&self, key: &str) -> Option<&ParamDef> {
        self.alias_index
            .get(&key.to_ascii_lowercase())
            .map(|&i| &self.params[i])
    }

    /// Return all registered parameter definitions.
    pub fn all_params(&self) -> &[ParamDef] {
        self.params
    }

    /// Check if a key is known (canonical or alias).
    pub fn is_known(&self, key: &str) -> bool {
        self.alias_index.contains_key(&key.to_ascii_lowercase())
    }
}

static REGISTRY: Lazy<ParamRegistry> = Lazy::new(|| ParamRegistry::new(PARAM_DEFS));

/// Global registry accessor.
pub fn registry() -> &'static ParamRegistry {
    &REGISTRY
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_aliases_to_canonical() {
        let r = registry();
        let cases: &[(&str, &str)] = &[
            ("SERVER", "host"),
            ("HOST", "host"),
            ("UID", "user"),
            ("PWD", "password"),
            ("PORT", "port"),
            ("PROTOCOL", "protocol"),
            ("ACCOUNT", "account"),
            ("DATABASE", "database"),
            ("SCHEMA", "schema"),
            ("WAREHOUSE", "warehouse"),
            ("ROLE", "role"),
            ("AUTHENTICATOR", "authenticator"),
            ("PRIV_KEY_FILE", "private_key_file"),
            ("PRIV_KEY_BASE64", "private_key"),
            ("PRIV_KEY_FILE_PWD", "private_key_password"),
            ("PRIV_KEY_PWD", "private_key_password"),
            ("TOKEN", "token"),
            ("TLS_CUSTOM_ROOT_STORE_PATH", "custom_root_store_path"),
            ("TLS_VERIFY_HOSTNAME", "verify_hostname"),
            ("TLS_VERIFY_CERTIFICATES", "verify_certificates"),
            ("CRL_MODE", "crl_check_mode"),
            ("CRL_ENABLED", "crl_check_mode"),
        ];
        for (alias, expected_canonical) in cases {
            let def = r
                .resolve(alias)
                .unwrap_or_else(|| panic!("alias {alias:?} should resolve"));
            assert_eq!(
                def.canonical_name, *expected_canonical,
                "alias {alias:?} resolved to {:?}, expected {expected_canonical:?}",
                def.canonical_name
            );
        }
    }

    #[test]
    fn resolve_canonical_names() {
        let r = registry();
        for param in r.all_params() {
            assert!(
                r.resolve(param.canonical_name).is_some(),
                "canonical name {:?} should resolve",
                param.canonical_name
            );
        }
    }

    #[test]
    fn unknown_key_returns_none() {
        let r = registry();
        assert!(r.resolve("nonexistent_param").is_none());
        assert!(r.resolve("").is_none());
        assert!(r.resolve("FOOBAR").is_none());
        assert!(!r.is_known("nonexistent_param"));
    }

    #[test]
    fn case_insensitive_lookup() {
        let r = registry();
        let variants = ["Host", "HOST", "host", "hOsT"];
        for key in variants {
            let def = r
                .resolve(key)
                .unwrap_or_else(|| panic!("{key:?} should resolve"));
            assert_eq!(def.canonical_name, "host");
        }
    }

    #[test]
    fn canonical_names_are_unique() {
        let r = registry();
        let mut seen = std::collections::HashSet::new();
        for param in r.all_params() {
            assert!(
                seen.insert(param.canonical_name),
                "duplicate canonical name: {:?}",
                param.canonical_name
            );
        }
    }

    #[test]
    fn no_alias_collides_with_another_canonical_name() {
        let r = registry();
        let canonical_set: std::collections::HashSet<&str> =
            r.all_params().iter().map(|p| p.canonical_name).collect();

        for param in r.all_params() {
            for alias in param.aliases {
                let lower = alias.to_ascii_lowercase();
                if canonical_set.contains(lower.as_str()) {
                    assert_eq!(
                        param.canonical_name, lower,
                        "alias {alias:?} of {:?} collides with canonical name {lower:?}",
                        param.canonical_name
                    );
                }
            }
        }
    }

    #[test]
    fn is_known_works() {
        let r = registry();
        assert!(r.is_known("account"));
        assert!(r.is_known("ACCOUNT"));
        assert!(r.is_known("SERVER"));
        assert!(r.is_known("host"));
        assert!(!r.is_known("unknown_key"));
    }
}
