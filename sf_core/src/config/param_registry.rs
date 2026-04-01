use std::collections::HashMap;
use std::fmt;

use std::sync::LazyLock;

use crate::config::settings::Setting;

/// A strongly-typed wrapper around a canonical parameter name.
///
/// Provides compile-time safety over bare `&str` keys while remaining
/// zero-cost at runtime (it is `Copy` and stores a `&'static str`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ParamKey(pub(crate) &'static str);

impl ParamKey {
    pub const fn as_str(&self) -> &'static str {
        self.0
    }
}

impl fmt::Display for ParamKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl From<ParamKey> for String {
    fn from(key: ParamKey) -> String {
        key.0.to_owned()
    }
}

impl AsRef<str> for ParamKey {
    fn as_ref(&self) -> &str {
        self.0
    }
}

/// Canonical parameter name constants.
///
/// Use these instead of bare string literals when referencing parameter names
/// in production code.  This gives compile-time typo detection and
/// find-all-references support.
pub mod param_names {
    use super::ParamKey;

    pub const ACCOUNT: ParamKey = ParamKey("account");
    pub const HOST: ParamKey = ParamKey("host");
    pub const PORT: ParamKey = ParamKey("port");
    pub const PROTOCOL: ParamKey = ParamKey("protocol");
    pub const SERVER_URL: ParamKey = ParamKey("server_url");
    pub const PRESERVE_UNDERSCORES_IN_HOSTNAME: ParamKey =
        ParamKey("preserve_underscores_in_hostname");
    pub const USER: ParamKey = ParamKey("user");
    pub const PASSWORD: ParamKey = ParamKey("password");
    pub const AUTHENTICATOR: ParamKey = ParamKey("authenticator");
    pub const PRIVATE_KEY: ParamKey = ParamKey("private_key");
    pub const PRIVATE_KEY_FILE: ParamKey = ParamKey("private_key_file");
    pub const PRIVATE_KEY_PASSWORD: ParamKey = ParamKey("private_key_password");
    pub const TOKEN: ParamKey = ParamKey("token");
    pub const PASSCODE: ParamKey = ParamKey("passcode");
    pub const PASSCODE_IN_PASSWORD: ParamKey = ParamKey("passcodeInPassword");
    pub const CLIENT_STORE_TEMPORARY_CREDENTIAL: ParamKey =
        ParamKey("client_store_temporary_credential");
    pub const DATABASE: ParamKey = ParamKey("database");
    pub const SCHEMA: ParamKey = ParamKey("schema");
    pub const WAREHOUSE: ParamKey = ParamKey("warehouse");
    pub const ROLE: ParamKey = ParamKey("role");
    pub const CONNECTION_NAME: ParamKey = ParamKey("connection_name");
    pub const CUSTOM_ROOT_STORE_PATH: ParamKey = ParamKey("custom_root_store_path");
    pub const VERIFY_HOSTNAME: ParamKey = ParamKey("verify_hostname");
    pub const VERIFY_CERTIFICATES: ParamKey = ParamKey("verify_certificates");
    pub const CRL_CHECK_MODE: ParamKey = ParamKey("crl_check_mode");
    pub const CRL_ENABLE_DISK_CACHING: ParamKey = ParamKey("crl_enable_disk_caching");
    pub const CRL_ENABLE_MEMORY_CACHING: ParamKey = ParamKey("crl_enable_memory_caching");
    pub const CRL_CACHE_DIR: ParamKey = ParamKey("crl_cache_dir");
    pub const CRL_VALIDITY_TIME: ParamKey = ParamKey("crl_validity_time");
    pub const CRL_ALLOW_CERTIFICATES_WITHOUT_CRL_URL: ParamKey =
        ParamKey("crl_allow_certificates_without_crl_url");
    pub const CRL_HTTP_TIMEOUT: ParamKey = ParamKey("crl_http_timeout");
    pub const CRL_CONNECTION_TIMEOUT: ParamKey = ParamKey("crl_connection_timeout");
    pub const ASYNC_EXECUTION: ParamKey = ParamKey("async_execution");
    pub const AUTHENTICATION_TIMEOUT: ParamKey = ParamKey("authentication_timeout");
    pub const OKTA_USERNAME: ParamKey = ParamKey("okta_username");
    pub const DISABLE_SAML_URL_CHECK: ParamKey = ParamKey("disable_saml_url_check");
}

/// Which API layer owns writes for a parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamScope {
    Connection,
    Session,
    Statement,
}

/// Defines a single supported configuration parameter.
pub struct ParamDef {
    /// The canonical key name used internally (e.g. `"host"`).
    pub canonical_name: &'static str,

    /// Alternative names accepted from wrappers (case-insensitive lookup).
    /// e.g. `&["SERVER", "HOST"]` all resolve to `"host"`.
    // TODO(sfc-gh-boler): Separate allowed aliases per wrapper. Implement this once
    //     wrapper identity available in core.
    pub aliases: &'static [&'static str],

    /// Primary expected value type.
    pub value_type: ValueType,

    /// Additional accepted value type when a wrapper legitimately sends a
    /// second representation for the same parameter.
    pub additional_value_type: Option<ValueType>,

    /// When this parameter is required.
    pub required: Required,

    /// Default value factory, if any.
    pub default: Option<fn() -> Setting>,

    /// Whether the value contains secrets (for log redaction).
    pub sensitive: bool,

    /// Human-readable description.
    pub description: &'static str,

    /// If deprecated, the canonical name of the replacement parameter.
    pub deprecated_by: Option<&'static str>,

    /// Which API layer owns writes for this parameter.
    pub scope: ParamScope,

    /// When true, the resolved connection-seed value participates in login / new session.
    pub used_at_connect: bool,

    /// When false, connection-level setters must reject changes once connected.
    pub mutable_after_connect: bool,
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

static PARAM_DEFS: &[ParamDef] = &[
    // ── Server ──────────────────────────────────────────────────────────
    ParamDef {
        canonical_name: param_names::ACCOUNT.as_str(),
        aliases: &["ACCOUNT"],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Always,
        default: None,
        sensitive: false,
        description: "Snowflake account identifier",
        deprecated_by: None,
        scope: ParamScope::Connection,
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::HOST.as_str(),
        aliases: &["SERVER", "HOST"],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Snowflake server hostname",
        deprecated_by: None,
        scope: ParamScope::Connection,
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::PORT.as_str(),
        aliases: &["PORT"],
        value_type: ValueType::Int,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Server port number",
        deprecated_by: None,
        scope: ParamScope::Connection,
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::PROTOCOL.as_str(),
        aliases: &["PROTOCOL"],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: Some(|| Setting::String("https".to_string())),
        sensitive: false,
        description: "Connection protocol (http or https)",
        deprecated_by: None,
        scope: ParamScope::Connection,
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::SERVER_URL.as_str(),
        aliases: &[],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Full server URL (alternative to host/port/protocol)",
        deprecated_by: None,
        scope: ParamScope::Connection,
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::PRESERVE_UNDERSCORES_IN_HOSTNAME.as_str(),
        aliases: &["ALLOWUNDERSCORESINHOST"],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: Some(|| Setting::Bool(false)),
        sensitive: false,
        description: "Preserve underscores in the hostname derived from the account name",
        deprecated_by: None,
        scope: ParamScope::Connection,
        used_at_connect: true,
        mutable_after_connect: false,
    },
    // ── Auth ────────────────────────────────────────────────────────────
    ParamDef {
        canonical_name: param_names::USER.as_str(),
        aliases: &["UID"],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Always,
        default: None,
        sensitive: false,
        description: "Login username",
        deprecated_by: None,
        scope: ParamScope::Connection,
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::PASSWORD.as_str(),
        aliases: &["PWD"],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::WhenAuthMethod("SNOWFLAKE_PASSWORD"),
        default: None,
        sensitive: true,
        description: "Login password",
        deprecated_by: None,
        scope: ParamScope::Connection,
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::AUTHENTICATOR.as_str(),
        aliases: &["AUTHENTICATOR"],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Authentication method (SNOWFLAKE_PASSWORD, SNOWFLAKE_JWT, PROGRAMMATIC_ACCESS_TOKEN, USERNAME_PASSWORD_MFA)",
        deprecated_by: None,
        scope: ParamScope::Connection,
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::PRIVATE_KEY.as_str(),
        aliases: &["PRIV_KEY_BASE64"],
        value_type: ValueType::String,
        additional_value_type: Some(ValueType::Bytes),
        required: Required::WhenAuthMethod("SNOWFLAKE_JWT"),
        default: None,
        sensitive: true,
        description: "Private key for key-pair authentication (base64-encoded or PEM)",
        deprecated_by: None,
        scope: ParamScope::Connection,
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::PRIVATE_KEY_FILE.as_str(),
        aliases: &["PRIV_KEY_FILE"],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Path to private key file for key-pair authentication",
        deprecated_by: None,
        scope: ParamScope::Connection,
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::PRIVATE_KEY_PASSWORD.as_str(),
        aliases: &["PRIV_KEY_FILE_PWD", "PRIV_KEY_PWD"],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: true,
        description: "Passphrase for encrypted private key",
        deprecated_by: None,
        scope: ParamScope::Connection,
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::TOKEN.as_str(),
        aliases: &["TOKEN"],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::WhenAuthMethod("PROGRAMMATIC_ACCESS_TOKEN"),
        default: None,
        sensitive: true,
        description: "Programmatic access token",
        deprecated_by: None,
        scope: ParamScope::Connection,
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::PASSCODE.as_str(),
        aliases: &["PASSCODE"],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: true,
        description: "MFA passcode for USERNAME_PASSWORD_MFA authentication",
        deprecated_by: None,
        scope: ParamScope::Connection,
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::PASSCODE_IN_PASSWORD.as_str(),
        aliases: &["PASSCODE_IN_PASSWORD"],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: Some(|| Setting::Bool(false)),
        sensitive: false,
        description: "Whether the MFA passcode is appended to the password",
        deprecated_by: None,
        scope: ParamScope::Connection,
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::CLIENT_STORE_TEMPORARY_CREDENTIAL.as_str(),
        aliases: &["clientStoreTemporaryCredential"],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: Some(|| Setting::Bool(false)),
        sensitive: false,
        description: "Enable MFA token caching for USERNAME_PASSWORD_MFA authentication",
        deprecated_by: None,
        scope: ParamScope::Connection,
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::AUTHENTICATION_TIMEOUT.as_str(),
        aliases: &[],
        value_type: ValueType::Int,
        additional_value_type: None,
        required: Required::Never,
        default: Some(|| Setting::Int(120)),
        sensitive: false,
        description: "Timeout in seconds for native Okta SSO authentication",
        deprecated_by: None,
        scope: ParamScope::Connection,
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::OKTA_USERNAME.as_str(),
        aliases: &[],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Okta username (defaults to the Snowflake user if omitted)",
        deprecated_by: None,
        scope: ParamScope::Connection,
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::DISABLE_SAML_URL_CHECK.as_str(),
        aliases: &[],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: Some(|| Setting::Bool(false)),
        sensitive: false,
        description: "Skip the Okta SAML URL host-match safety check",
        deprecated_by: None,
        scope: ParamScope::Connection,
        used_at_connect: true,
        mutable_after_connect: false,
    },
    // ── Session ─────────────────────────────────────────────────────────
    ParamDef {
        canonical_name: param_names::DATABASE.as_str(),
        aliases: &["DATABASE"],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Default database to use",
        deprecated_by: None,
        scope: ParamScope::Session,
        used_at_connect: true,
        mutable_after_connect: true,
    },
    ParamDef {
        canonical_name: param_names::SCHEMA.as_str(),
        aliases: &["SCHEMA"],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Default schema to use",
        deprecated_by: None,
        scope: ParamScope::Session,
        used_at_connect: true,
        mutable_after_connect: true,
    },
    ParamDef {
        canonical_name: param_names::WAREHOUSE.as_str(),
        aliases: &["WAREHOUSE"],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Default warehouse to use",
        deprecated_by: None,
        scope: ParamScope::Session,
        used_at_connect: true,
        mutable_after_connect: true,
    },
    ParamDef {
        canonical_name: param_names::ROLE.as_str(),
        aliases: &["ROLE"],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Default role to use",
        deprecated_by: None,
        scope: ParamScope::Session,
        used_at_connect: true,
        mutable_after_connect: true,
    },
    // ── TLS ─────────────────────────────────────────────────────────────
    ParamDef {
        canonical_name: param_names::CUSTOM_ROOT_STORE_PATH.as_str(),
        aliases: &["TLS_CUSTOM_ROOT_STORE_PATH"],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Path to custom root certificate store",
        deprecated_by: None,
        scope: ParamScope::Connection,
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::VERIFY_HOSTNAME.as_str(),
        aliases: &["TLS_VERIFY_HOSTNAME"],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: Some(|| Setting::Bool(true)),
        sensitive: false,
        description: "Whether to verify the server hostname in TLS",
        deprecated_by: None,
        scope: ParamScope::Connection,
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::VERIFY_CERTIFICATES.as_str(),
        aliases: &["TLS_VERIFY_CERTIFICATES"],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: Some(|| Setting::Bool(true)),
        sensitive: false,
        description: "Whether to verify TLS certificates",
        deprecated_by: None,
        scope: ParamScope::Connection,
        used_at_connect: true,
        mutable_after_connect: false,
    },
    // ── CRL ─────────────────────────────────────────────────────────────
    ParamDef {
        canonical_name: param_names::CRL_CHECK_MODE.as_str(),
        aliases: &["CRL_MODE", "CRL_ENABLED"],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: Some(|| Setting::String("DISABLED".to_string())),
        sensitive: false,
        description: "Certificate revocation check mode (DISABLED, ENABLED, ADVISORY)",
        deprecated_by: None,
        scope: ParamScope::Connection,
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::CRL_ENABLE_DISK_CACHING.as_str(),
        aliases: &[],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: Some(|| Setting::Bool(true)),
        sensitive: false,
        description: "Enable disk caching for CRL responses",
        deprecated_by: None,
        scope: ParamScope::Connection,
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::CRL_ENABLE_MEMORY_CACHING.as_str(),
        aliases: &[],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: Some(|| Setting::Bool(true)),
        sensitive: false,
        description: "Enable in-memory caching for CRL responses",
        deprecated_by: None,
        scope: ParamScope::Connection,
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::CRL_CACHE_DIR.as_str(),
        aliases: &[],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Directory for CRL cache files",
        deprecated_by: None,
        scope: ParamScope::Connection,
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::CRL_VALIDITY_TIME.as_str(),
        aliases: &[],
        value_type: ValueType::Int,
        additional_value_type: None,
        required: Required::Never,
        default: Some(|| Setting::Int(10)),
        sensitive: false,
        description: "CRL cache validity time in days",
        deprecated_by: None,
        scope: ParamScope::Connection,
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::CRL_ALLOW_CERTIFICATES_WITHOUT_CRL_URL.as_str(),
        aliases: &[],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: Some(|| Setting::Bool(false)),
        sensitive: false,
        description: "Allow certificates that do not include a CRL distribution URL",
        deprecated_by: None,
        scope: ParamScope::Connection,
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::CRL_HTTP_TIMEOUT.as_str(),
        aliases: &[],
        value_type: ValueType::Int,
        additional_value_type: None,
        required: Required::Never,
        default: Some(|| Setting::Int(30)),
        sensitive: false,
        description: "HTTP timeout in seconds for CRL endpoint requests",
        deprecated_by: None,
        scope: ParamScope::Connection,
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::CRL_CONNECTION_TIMEOUT.as_str(),
        aliases: &[],
        value_type: ValueType::Int,
        additional_value_type: None,
        required: Required::Never,
        default: Some(|| Setting::Int(10)),
        sensitive: false,
        description: "Connection timeout in seconds for CRL endpoints",
        deprecated_by: None,
        scope: ParamScope::Connection,
        used_at_connect: true,
        mutable_after_connect: false,
    },
    // ── Client ──────────────────────────────────────────────────────────
    ParamDef {
        canonical_name: param_names::CONNECTION_NAME.as_str(),
        aliases: &[],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Named connection to load from TOML configuration files",
        deprecated_by: None,
        scope: ParamScope::Connection,
        used_at_connect: false,
        mutable_after_connect: false,
    },
    // ── Statement ──────────────────────────────────────────────────────
    ParamDef {
        canonical_name: param_names::ASYNC_EXECUTION.as_str(),
        aliases: &[],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: Some(|| Setting::Bool(false)),
        sensitive: false,
        description: "Execute queries asynchronously",
        deprecated_by: None,
        scope: ParamScope::Statement,
        used_at_connect: false,
        mutable_after_connect: true,
    },
];

impl ParamDef {
    /// Whether the resolved value may participate in login / new session creation.
    ///
    /// [`ParamScope::Statement`] parameters are never consumed at connect regardless
    /// of stored metadata.
    #[inline]
    pub fn effective_used_at_connect(&self) -> bool {
        if self.scope == ParamScope::Statement {
            return false;
        }
        self.used_at_connect
    }
}

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
    ///
    /// Accepts any type that can be viewed as a string — `ParamKey`, `&str`,
    /// or `String` — so callers with a typed key can pass it directly without
    /// calling `.as_str()`.  Lookup is case-insensitive.
    pub fn resolve(&self, key: impl AsRef<str>) -> Option<&ParamDef> {
        self.alias_index
            .get(&key.as_ref().to_ascii_lowercase())
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

static REGISTRY: LazyLock<ParamRegistry> = LazyLock::new(|| ParamRegistry::new(PARAM_DEFS));

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
            ("PASSCODE", "passcode"),
            ("PASSCODE_IN_PASSWORD", "passcodeInPassword"),
            (
                "clientStoreTemporaryCredential",
                "client_store_temporary_credential",
            ),
            ("TLS_CUSTOM_ROOT_STORE_PATH", "custom_root_store_path"),
            ("TLS_VERIFY_HOSTNAME", "verify_hostname"),
            ("TLS_VERIFY_CERTIFICATES", "verify_certificates"),
            ("CRL_MODE", "crl_check_mode"),
            ("CRL_ENABLED", "crl_check_mode"),
            ("ALLOWUNDERSCORESINHOST", "preserve_underscores_in_hostname"),
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

    #[test]
    fn statement_scope_params_are_never_used_at_connect() {
        let r = registry();
        for p in r.all_params() {
            if p.scope == ParamScope::Statement {
                assert!(
                    !p.used_at_connect,
                    "expected used_at_connect == false for {}",
                    p.canonical_name
                );
                assert!(!p.effective_used_at_connect());
            }
        }
    }

    #[test]
    fn session_context_params_are_session_scoped_and_mutable_after_connect() {
        let r = registry();
        for key in ["database", "schema", "warehouse", "role"] {
            let d = r
                .resolve(key)
                .unwrap_or_else(|| panic!("expected registry entry for {key}"));
            assert_eq!(d.scope, ParamScope::Session, "key {key}");
            assert!(d.used_at_connect, "key {key}");
            assert!(d.mutable_after_connect, "key {key}");
        }
    }
}
