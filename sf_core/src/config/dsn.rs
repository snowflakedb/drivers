//! Parse a Snowflake DSN connection string into sf_core connection parameters.
//!
//! Supports `user[:password]@<account>/db/schema`, `.../db`, and
//! `user[:password]@host:port/db/schema?account=<account>` forms, with an
//! optional case-insensitive `snowflake://` prefix. Only parameters sf_core
//! recognizes (per [`crate::config::param_registry`]) are emitted; every other
//! key is dropped into [`ParsedDsn::warnings`]. Host derivation from `account`
//! is left to [`crate::config::resolver`].

use std::collections::HashMap;

use snafu::{Location, ResultExt, Snafu};

use crate::config::param_registry::{ParamKey, param_names};
use crate::config::settings::Setting;

/// Outcome of [`parse_dsn`]: recognized params plus notes for anything dropped.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedDsn {
    /// Canonical-keyed connection params, ready for `connection_set_options`.
    pub params: HashMap<String, Setting>,
    /// One note per dropped key (unsupported/unknown key, `region`, OCSP param,
    /// unparseable boolean) or query override. Names the key only, never a value.
    pub warnings: Vec<String>,
}

#[derive(Debug, Snafu, error_trace::ErrorTrace)]
#[snafu(visibility(pub(crate)))]
pub enum DsnError {
    /// The connection string is empty after trimming and prefix removal.
    #[snafu(display("connection string is empty"))]
    Empty {
        #[snafu(implicit)]
        location: Location,
    },
    /// A URL scheme other than `snowflake://` was supplied.
    #[snafu(display(
        "unsupported connection-string scheme {scheme:?}; only 'snowflake://' is accepted"
    ))]
    UnsupportedScheme {
        scheme: String,
        #[snafu(implicit)]
        location: Location,
    },
    /// Neither an account nor a host could be determined.
    #[snafu(display("connection string specifies neither an account nor a host"))]
    MissingAccountAndHost {
        #[snafu(implicit)]
        location: Location,
    },
    /// The `host:port` authority carried a non-numeric or out-of-range port.
    /// The offending value is deliberately excluded so a mistyped credential
    /// (e.g. `user:secret` with no host) can never leak through the error.
    #[snafu(display("invalid port in connection string"))]
    InvalidPort {
        source: std::num::ParseIntError,
        #[snafu(implicit)]
        location: Location,
    },
    /// A component failed percent-decoding; `component` names it, so no
    /// credential value is ever included in the error.
    #[snafu(display("failed to decode connection-string {component}"))]
    PercentDecode {
        component: &'static str,
        source: std::string::FromUtf8Error,
        #[snafu(implicit)]
        location: Location,
    },
}

/// Parse a Snowflake DSN connection string into canonical sf_core params. An
/// optional, case-insensitive `snowflake://` prefix yields identical output.
pub fn parse_dsn(dsn: &str) -> Result<ParsedDsn, DsnError> {
    let stripped = strip_optional_scheme(dsn.trim())?;
    if stripped.is_empty() {
        return EmptySnafu.fail();
    }

    let no_fragment = stripped.split('#').next().unwrap_or(stripped);
    let (main, query) = match no_fragment.split_once('?') {
        Some((m, q)) => (m, Some(q)),
        None => (no_fragment, None),
    };

    let mut params: HashMap<String, Setting> = HashMap::new();
    let mut warnings: Vec<String> = Vec::new();

    let (userinfo, host_and_path) = match main.rfind('@') {
        Some(at) => (Some(&main[..at]), &main[at + 1..]),
        None => (None, main),
    };

    if let Some(ui) = userinfo {
        match ui.split_once(':') {
            Some((u, pw)) => {
                set_str(&mut params, param_names::USER, &query_unescape("user", u)?);
                set_str(
                    &mut params,
                    param_names::PASSWORD,
                    &query_unescape("password", pw)?,
                );
            }
            None => set_str(&mut params, param_names::USER, &query_unescape("user", ui)?),
        }
    }

    let (authority, path) = match host_and_path.split_once('/') {
        Some((a, p)) => (a, Some(p)),
        None => (host_and_path, None),
    };

    if let Some((host, port_str)) = authority.split_once(':') {
        let port: u16 = port_str.parse().context(InvalidPortSnafu)?;
        if !host.is_empty() {
            set_str(&mut params, param_names::HOST, host);
        }
        params.insert(param_names::PORT.into(), Setting::Int(i64::from(port)));
    } else if !authority.is_empty() {
        set_str(&mut params, param_names::ACCOUNT, authority);
    }

    if let Some(path) = path {
        let mut segs = path.splitn(3, '/');
        if let Some(db) = segs.next().filter(|s| !s.is_empty()) {
            set_str(
                &mut params,
                param_names::DATABASE,
                &query_unescape("database", db)?,
            );
        }
        if let Some(schema) = segs.next().filter(|s| !s.is_empty()) {
            set_str(
                &mut params,
                param_names::SCHEMA,
                &query_unescape("schema", schema)?,
            );
        }
    }

    if let Some(query) = query {
        for pair in query.split('&') {
            if pair.is_empty() {
                continue;
            }
            let Some((key, raw_value)) = pair.split_once('=') else {
                warnings.push(
                    "dropped a malformed connection-string parameter (missing '='); content omitted in case it is sensitive".to_string(),
                );
                continue;
            };
            let value = query_unescape("parameter value", raw_value)?;
            apply_query_param(&mut params, &mut warnings, key, &value);
        }
    }

    if !params.contains_key(param_names::ACCOUNT.as_str())
        && !params.contains_key(param_names::HOST.as_str())
    {
        return MissingAccountAndHostSnafu.fail();
    }

    Ok(ParsedDsn { params, warnings })
}

/// Returns the remainder after a `snowflake://` prefix (case-insensitive), the
/// input unchanged when there is no `scheme://`, or an error for any other
/// scheme. A bare `snowflake:` (no `//`) is left intact as userinfo.
fn strip_optional_scheme(s: &str) -> Result<&str, DsnError> {
    let Some(idx) = s.find("://") else {
        return Ok(s);
    };
    let scheme = &s[..idx];
    let is_scheme = scheme
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-'));
    if !is_scheme {
        // A `://` that follows non-scheme characters (e.g. inside a query
        // value) is not a scheme separator; parse as a schemeless DSN.
        return Ok(s);
    }
    if scheme.eq_ignore_ascii_case("snowflake") {
        Ok(&s[idx + 3..])
    } else {
        UnsupportedSchemeSnafu {
            scheme: scheme.to_string(),
        }
        .fail()
    }
}

/// URL-unescape one component with `QueryUnescape` semantics: `+` → space, then
/// percent-decode.
fn query_unescape(component: &'static str, s: &str) -> Result<String, DsnError> {
    let plus_as_space = s.replace('+', " ");
    urlencoding::decode(&plus_as_space)
        .map(|cow| cow.into_owned())
        .context(PercentDecodeSnafu { component })
}

fn set_str(params: &mut HashMap<String, Setting>, key: ParamKey, value: &str) {
    params.insert(key.into(), Setting::String(value.to_string()));
}

/// Sets a value from the query, warning when it overrides one already taken
/// from the DSN path/authority.
fn set_override(
    params: &mut HashMap<String, Setting>,
    warnings: &mut Vec<String>,
    key: ParamKey,
    dsn_key: &str,
    value: &str,
) {
    if value.is_empty() {
        warnings.push(format!("ignored empty query parameter `{dsn_key}`"));
        return;
    }
    if params.contains_key(key.as_str()) {
        warnings.push(format!(
            "query parameter `{dsn_key}` overrides the value from the connection-string path/authority"
        ));
    }
    set_str(params, key, value);
}

/// Stores a parsed boolean under `key`, flipping polarity when `invert` is set;
/// warns on an unparseable boolean.
fn set_bool(
    params: &mut HashMap<String, Setting>,
    warnings: &mut Vec<String>,
    key: ParamKey,
    dsn_key: &str,
    value: &str,
    invert: bool,
) {
    match parse_bool(value) {
        Some(b) => {
            params.insert(key.into(), Setting::Bool(b ^ invert));
        }
        None => warnings.push(format!("dropped `{dsn_key}`: value is not a valid boolean")),
    }
}

/// The boolean token set accepted by the reference DSN parser (`ParseBool`).
fn parse_bool(s: &str) -> Option<bool> {
    match s {
        "1" | "t" | "T" | "TRUE" | "true" | "True" => Some(true),
        "0" | "f" | "F" | "FALSE" | "false" | "False" => Some(false),
        _ => None,
    }
}

/// Map one DSN query key/value onto sf_core's canonical set, or drop it with a
/// warning when sf_core has no equivalent.
fn apply_query_param(
    params: &mut HashMap<String, Setting>,
    warnings: &mut Vec<String>,
    key: &str,
    value: &str,
) {
    use param_names as p;
    match key {
        "account" => set_override(params, warnings, p::ACCOUNT, "account", value),
        "database" => set_override(params, warnings, p::DATABASE, "database", value),
        "schema" => set_override(params, warnings, p::SCHEMA, "schema", value),

        "warehouse" => set_str(params, p::WAREHOUSE, value),
        "role" => set_str(params, p::ROLE, value),
        "protocol" => set_str(params, p::PROTOCOL, value),
        "passcode" => set_str(params, p::PASSCODE, value),
        "token" => set_str(params, p::TOKEN, value),
        "application" => set_str(params, p::APPLICATION, value),
        "privateKey" => set_str(params, p::PRIVATE_KEY, value),
        "oauthClientId" => set_str(params, p::OAUTH_CLIENT_ID, value),
        "oauthClientSecret" => set_str(params, p::OAUTH_CLIENT_SECRET, value),
        "oauthAuthorizationUrl" => set_str(params, p::OAUTH_AUTHORIZATION_URL, value),
        "oauthTokenRequestUrl" => set_str(params, p::OAUTH_TOKEN_REQUEST_URL, value),
        "oauthRedirectUri" => set_str(params, p::OAUTH_REDIRECT_URI, value),
        "oauthScope" => set_str(params, p::OAUTH_SCOPE, value),
        "workloadIdentityProvider" => set_str(params, p::WORKLOAD_IDENTITY_PROVIDER, value),
        "workloadIdentityEntraResource" => {
            set_str(params, p::WORKLOAD_IDENTITY_ENTRA_RESOURCE, value)
        }
        "workloadIdentityImpersonationPath" => {
            set_str(params, p::WORKLOAD_IDENTITY_IMPERSONATION_PATH, value)
        }
        "proxyHost" => set_str(params, p::PROXY_HOST, value),
        "proxyPort" => set_str(params, p::PROXY_PORT, value),
        "proxyUser" => set_str(params, p::PROXY_USER, value),
        "proxyPassword" => set_str(params, p::PROXY_PASSWORD, value),
        "noProxy" => set_str(params, p::NO_PROXY, value),
        "connectionDiagnosticsAllowlistFile" => {
            set_str(params, p::CONNECTION_DIAG_ALLOWLIST_PATH, value)
        }

        "loginTimeout" => set_str(params, p::LOGIN_TIMEOUT, value),
        "requestTimeout" => set_str(params, p::QUERY_TIMEOUT, value),
        "crlHttpClientTimeout" => set_str(params, p::CRL_HTTP_TIMEOUT, value),
        "crlDownloadMaxSize" => set_str(params, p::CRL_MAX_DOWNLOAD_SIZE, value),

        "passcodeInPassword" => {
            set_bool(params, warnings, p::PASSCODE_IN_PASSWORD, key, value, false)
        }
        "enableSingleUseRefreshTokens" => set_bool(
            params,
            warnings,
            p::OAUTH_ENABLE_SINGLE_USE_REFRESH_TOKENS,
            key,
            value,
            false,
        ),
        "validateDefaultParameters" => {
            set_bool(params, warnings, p::VALIDATE_DEFAULT_PARAMETERS, key, value, false)
        }
        "clientStoreTemporaryCredential" => set_bool(
            params,
            warnings,
            p::CLIENT_STORE_TEMPORARY_CREDENTIAL,
            key,
            value,
            false,
        ),
        "logQueryText" => set_bool(params, warnings, p::LOG_QUERY_TEXT, key, value, false),
        "logQueryParameters" => {
            set_bool(params, warnings, p::LOG_QUERY_PARAMETERS, key, value, false)
        }
        "disableSamlURLCheck" => {
            set_bool(params, warnings, p::DISABLE_SAML_URL_CHECK, key, value, false)
        }
        "serverSessionKeepAlive" => {
            set_bool(params, warnings, p::SERVER_SESSION_KEEP_ALIVE, key, value, false)
        }
        "crlAllowCertificatesWithoutCrlURL" => set_bool(
            params,
            warnings,
            p::CRL_ALLOW_CERTIFICATES_WITHOUT_CRL_URL,
            key,
            value,
            false,
        ),
        "connectionDiagnosticsEnabled" => {
            set_bool(params, warnings, p::ENABLE_CONNECTION_DIAG, key, value, false)
        }
        "disableConsoleLogin" => {
            set_bool(params, warnings, p::OAUTH_DISABLE_CONSOLE_LOGIN, key, value, false)
        }

        "singleAuthenticationPrompt" => {
            set_bool(params, warnings, p::DISABLE_PARALLEL_USER_PROMPT, key, value, false)
        }
        "crlInMemoryCacheDisabled" => {
            set_bool(params, warnings, p::CRL_ENABLE_MEMORY_CACHING, key, value, true)
        }
        "crlOnDiskCacheDisabled" => {
            set_bool(params, warnings, p::CRL_ENABLE_DISK_CACHING, key, value, true)
        }

        "authenticator" => {
            if value.eq_ignore_ascii_case("tokenaccessor") {
                warnings.push(
                    "dropped `authenticator=tokenaccessor`: no sf_core equivalent".to_string(),
                );
            } else {
                set_str(params, p::AUTHENTICATOR, value);
            }
        }

        "certRevocationCheckMode" => match value.to_ascii_lowercase().as_str() {
            "disabled" => set_str(params, p::CRL_CHECK_MODE, "DISABLED"),
            "advisory" => set_str(params, p::CRL_CHECK_MODE, "ADVISORY"),
            "enabled" => set_str(params, p::CRL_CHECK_MODE, "ENABLED"),
            _ => warnings.push(
                "dropped `certRevocationCheckMode`: value is not disabled/advisory/enabled"
                    .to_string(),
            ),
        },

        "region" => warnings.push(
            "dropped `region`: sf_core has no region parameter; encode the region in the account identifier (account.region)"
                .to_string(),
        ),

        other => warnings.push(format!("dropped unsupported connection-string parameter `{other}`")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(dsn: &str) -> ParsedDsn {
        parse_dsn(dsn).unwrap_or_else(|e| panic!("parse_dsn({dsn:?}) failed: {e}"))
    }

    fn get_str<'a>(p: &'a ParsedDsn, key: ParamKey) -> Option<&'a str> {
        match p.params.get(key.as_str()) {
            Some(Setting::String(s)) => Some(s.as_str()),
            _ => None,
        }
    }

    fn get_bool(p: &ParsedDsn, key: ParamKey) -> Option<bool> {
        match p.params.get(key.as_str()) {
            Some(Setting::Bool(b)) => Some(*b),
            _ => None,
        }
    }

    #[test]
    fn form1_full_account_db_schema_query() {
        let p =
            parse("jsmith:mypassword@my_organization-my_account/mydb/testschema?warehouse=mywh");
        assert_eq!(get_str(&p, param_names::USER), Some("jsmith"));
        assert_eq!(get_str(&p, param_names::PASSWORD), Some("mypassword"));
        assert_eq!(
            get_str(&p, param_names::ACCOUNT),
            Some("my_organization-my_account")
        );
        assert_eq!(get_str(&p, param_names::DATABASE), Some("mydb"));
        assert_eq!(get_str(&p, param_names::SCHEMA), Some("testschema"));
        assert_eq!(get_str(&p, param_names::WAREHOUSE), Some("mywh"));
        assert!(p.warnings.is_empty(), "warnings: {:?}", p.warnings);
    }

    #[test]
    fn form2_account_db_only() {
        let p = parse("user:pw@acct/db");
        assert_eq!(get_str(&p, param_names::ACCOUNT), Some("acct"));
        assert_eq!(get_str(&p, param_names::DATABASE), Some("db"));
        assert_eq!(p.params.get(param_names::SCHEMA.as_str()), None);
    }

    #[test]
    fn form3_host_port_with_account_query() {
        let p = parse("user:pass@host.example.com:443/db/schema?account=acme");
        assert_eq!(get_str(&p, param_names::HOST), Some("host.example.com"));
        assert_eq!(
            p.params.get(param_names::PORT.as_str()),
            Some(&Setting::Int(443))
        );
        assert_eq!(get_str(&p, param_names::ACCOUNT), Some("acme"));
        assert_eq!(get_str(&p, param_names::DATABASE), Some("db"));
        assert_eq!(get_str(&p, param_names::SCHEMA), Some("schema"));
    }

    #[test]
    fn optional_prefix_parity_and_case_insensitivity() {
        let bare = parse("user:pw@acct/db/sc?warehouse=wh");
        for prefixed in [
            "snowflake://user:pw@acct/db/sc?warehouse=wh",
            "SNOWFLAKE://user:pw@acct/db/sc?warehouse=wh",
            "Snowflake://user:pw@acct/db/sc?warehouse=wh",
        ] {
            let p = parse(prefixed);
            assert_eq!(p.params, bare.params, "params differ for {prefixed:?}");
            assert_eq!(
                p.warnings, bare.warnings,
                "warnings differ for {prefixed:?}"
            );
        }
    }

    #[test]
    fn bare_snowflake_colon_is_userinfo_not_scheme() {
        let p = parse("snowflake:pw@acct");
        assert_eq!(get_str(&p, param_names::USER), Some("snowflake"));
        assert_eq!(get_str(&p, param_names::PASSWORD), Some("pw"));
        assert_eq!(get_str(&p, param_names::ACCOUNT), Some("acct"));
    }

    #[test]
    fn other_scheme_is_rejected() {
        for dsn in ["http://acct/db", "mysql://user@acct/db"] {
            match parse_dsn(dsn) {
                Err(DsnError::UnsupportedScheme { scheme, .. }) => {
                    assert!(dsn.starts_with(&scheme), "scheme {scheme:?} for {dsn:?}");
                }
                other => panic!("expected UnsupportedScheme for {dsn:?}, got {other:?}"),
            }
        }
    }

    #[test]
    fn unescapes_credentials_and_path_and_plus_as_space() {
        let p = parse("user%40name:p%2Fw@acct/d+b?role=a+b");
        assert_eq!(get_str(&p, param_names::USER), Some("user@name"));
        assert_eq!(get_str(&p, param_names::PASSWORD), Some("p/w"));
        assert_eq!(get_str(&p, param_names::DATABASE), Some("d b"));
        assert_eq!(get_str(&p, param_names::ROLE), Some("a b"));
    }

    #[test]
    fn empty_password_with_colon() {
        let p = parse("user:@acct");
        assert_eq!(get_str(&p, param_names::USER), Some("user"));
        assert_eq!(get_str(&p, param_names::PASSWORD), Some(""));
    }

    #[test]
    fn dotted_account_authority_is_verbatim_not_truncated() {
        let p = parse("u:pw@acct.us-east-1/db");
        assert_eq!(get_str(&p, param_names::ACCOUNT), Some("acct.us-east-1"));
    }

    #[test]
    fn direct_and_translated_mappings() {
        let p = parse(
            "u:pw@acct?oauthClientId=abc&serverSessionKeepAlive=true&proxyHost=proxy&loginTimeout=30&requestTimeout=0&certRevocationCheckMode=advisory&disableConsoleLogin=true",
        );
        assert_eq!(get_str(&p, param_names::OAUTH_CLIENT_ID), Some("abc"));
        assert_eq!(
            get_bool(&p, param_names::SERVER_SESSION_KEEP_ALIVE),
            Some(true)
        );
        assert_eq!(get_str(&p, param_names::PROXY_HOST), Some("proxy"));
        assert_eq!(get_str(&p, param_names::LOGIN_TIMEOUT), Some("30"));
        assert_eq!(get_str(&p, param_names::QUERY_TIMEOUT), Some("0"));
        assert_eq!(get_str(&p, param_names::CRL_CHECK_MODE), Some("ADVISORY"));
        assert_eq!(
            get_bool(&p, param_names::OAUTH_DISABLE_CONSOLE_LOGIN),
            Some(true)
        );
        assert!(p.warnings.is_empty(), "warnings: {:?}", p.warnings);
    }

    #[test]
    fn inverted_crl_cache_booleans() {
        let p = parse("u:pw@acct?crlInMemoryCacheDisabled=true&crlOnDiskCacheDisabled=true");
        assert_eq!(
            get_bool(&p, param_names::CRL_ENABLE_MEMORY_CACHING),
            Some(false)
        );
        assert_eq!(
            get_bool(&p, param_names::CRL_ENABLE_DISK_CACHING),
            Some(false)
        );
    }

    #[test]
    fn single_authentication_prompt_maps_without_inversion() {
        let t = parse("u:pw@acct?singleAuthenticationPrompt=true");
        assert_eq!(
            get_bool(&t, param_names::DISABLE_PARALLEL_USER_PROMPT),
            Some(true)
        );
        let f = parse("u:pw@acct?singleAuthenticationPrompt=false");
        assert_eq!(
            get_bool(&f, param_names::DISABLE_PARALLEL_USER_PROMPT),
            Some(false)
        );
    }

    #[test]
    fn authenticator_passthrough_and_tokenaccessor_dropped() {
        let p = parse("u:pw@acct?authenticator=EXTERNALBROWSER");
        assert_eq!(
            get_str(&p, param_names::AUTHENTICATOR),
            Some("EXTERNALBROWSER")
        );

        let p2 = parse("u:pw@acct?authenticator=tokenaccessor");
        assert_eq!(p2.params.get(param_names::AUTHENTICATOR.as_str()), None);
        assert_eq!(p2.warnings.len(), 1);
    }

    #[test]
    fn unsupported_keys_dropped_with_warnings() {
        let p = parse(
            "u:pw@acct?disableOCSPChecks=true&ocspFailOpen=false&region=eu-central-1&tracing=x&FooBar=baz",
        );
        for key in [
            "disableOCSPChecks",
            "ocspFailOpen",
            "region",
            "tracing",
            "FooBar",
        ] {
            assert_eq!(p.params.get(key), None, "{key} must not be emitted");
        }
        assert_eq!(p.warnings.len(), 5, "warnings: {:?}", p.warnings);
        assert_eq!(get_str(&p, param_names::ACCOUNT), Some("acct"));
    }

    #[test]
    fn query_account_override_warns() {
        let p = parse("acct/db1/sch1?database=db2");
        assert_eq!(get_str(&p, param_names::DATABASE), Some("db2"));
        assert!(
            p.warnings.iter().any(|w| w.contains("database")),
            "expected override warning, got {:?}",
            p.warnings
        );
    }

    #[test]
    fn no_userinfo_account_from_query() {
        let p = parse("host:8080/db/sc?account=acme");
        assert_eq!(get_str(&p, param_names::HOST), Some("host"));
        assert_eq!(
            p.params.get(param_names::PORT.as_str()),
            Some(&Setting::Int(8080))
        );
        assert_eq!(get_str(&p, param_names::ACCOUNT), Some("acme"));
    }

    #[test]
    fn invalid_port_errors() {
        assert!(matches!(
            parse_dsn("u:pw@host:notaport/db"),
            Err(DsnError::InvalidPort { .. })
        ));
    }

    #[test]
    fn out_of_range_and_negative_ports_error() {
        assert!(matches!(
            parse_dsn("u:pw@host:70000/db"),
            Err(DsnError::InvalidPort { .. })
        ));
        assert!(matches!(
            parse_dsn("u:pw@host:-1/db"),
            Err(DsnError::InvalidPort { .. })
        ));
    }

    #[test]
    fn malformed_credential_like_port_is_redacted() {
        let err = parse_dsn("user:secret").unwrap_err();
        assert!(matches!(err, DsnError::InvalidPort { .. }));
        assert!(
            !format!("{err}").contains("secret"),
            "error must not leak the value: {err}"
        );
    }

    #[test]
    fn empty_account_query_does_not_satisfy_requirement() {
        assert!(matches!(
            parse_dsn("u:pw@?account="),
            Err(DsnError::MissingAccountAndHost { .. })
        ));
    }

    #[test]
    fn empty_account_override_does_not_clobber() {
        let p = parse("acct/db?account=");
        assert_eq!(get_str(&p, param_names::ACCOUNT), Some("acct"));
        assert!(p.warnings.iter().any(|w| w.contains("account")));
    }

    #[test]
    fn key_only_query_pair_is_dropped_without_leaking_content() {
        let p = parse("u:pw@acct?SuperSecretToken");
        assert!(
            p.warnings.iter().any(|w| w.contains("malformed")),
            "expected a malformed-parameter warning, got {:?}",
            p.warnings
        );
        assert!(
            !p.warnings.iter().any(|w| w.contains("SuperSecretToken")),
            "warning must not echo the raw segment: {:?}",
            p.warnings
        );
    }

    #[test]
    fn empty_and_prefix_only_and_missing_account() {
        assert!(matches!(parse_dsn(""), Err(DsnError::Empty { .. })));
        assert!(matches!(parse_dsn("   "), Err(DsnError::Empty { .. })));
        assert!(matches!(
            parse_dsn("snowflake://"),
            Err(DsnError::Empty { .. })
        ));
        assert!(matches!(
            parse_dsn("user:pw@/db"),
            Err(DsnError::MissingAccountAndHost { .. })
        ));
    }

    #[test]
    fn trailing_fragment_is_stripped() {
        let p = parse("u:pw@acct/db?warehouse=wh#frag");
        assert_eq!(get_str(&p, param_names::WAREHOUSE), Some("wh"));
        assert_eq!(get_str(&p, param_names::DATABASE), Some("db"));
    }

    #[test]
    fn every_emitted_key_resolves_in_registry() {
        let p = parse(
            "u:pw@acct/db/sc?warehouse=wh&role=r&oauthClientId=c&proxyHost=p&loginTimeout=1&serverSessionKeepAlive=true&certRevocationCheckMode=enabled&singleAuthenticationPrompt=true",
        );
        let registry = crate::config::param_registry::registry();
        for key in p.params.keys() {
            assert!(
                registry.resolve(key).is_some(),
                "emitted key {key:?} does not resolve in the param registry"
            );
        }
    }
}
