use std::fs;

use base64::{Engine as _, engine::general_purpose};
use openssl::pkey::PKey;

use crate::config::settings::{Setting, Settings};
use crate::config::toml_loader::{FilePermissionCheck, check_file_permissions};
use crate::config::{
    ConfigError, ConflictingParametersSnafu, InvalidParameterValueSnafu, MissingParameterSnafu,
};
use crate::sensitive::SensitiveString;

pub(super) fn der_to_pem(der_bytes: &[u8]) -> Result<SensitiveString, ConfigError> {
    let pkey = PKey::private_key_from_der(der_bytes).map_err(|e| {
        InvalidParameterValueSnafu {
            parameter: "private_key".to_string(),
            value: "(binary data)".to_string(),
            explanation: format!("Could not parse DER private key: {e}"),
        }
        .build()
    })?;

    let pem_bytes = pkey.private_key_to_pem_pkcs8().map_err(|e| {
        InvalidParameterValueSnafu {
            parameter: "private_key".to_string(),
            value: "(binary data)".to_string(),
            explanation: format!("Could not convert private key to PEM: {e}"),
        }
        .build()
    })?;

    String::from_utf8(pem_bytes)
        .map(SensitiveString::from)
        .map_err(|e| {
            InvalidParameterValueSnafu {
                parameter: "private_key".to_string(),
                value: "(binary data)".to_string(),
                explanation: format!("PEM output is not valid UTF-8: {e}"),
            }
            .build()
        })
}

/// Parse a private key from settings into a PEM string ready for JWT signing.
///
/// Accepts three forms for `private_key`:
///   - `Setting::Bytes` — raw DER bytes (Python connector path)
///   - `Setting::String` starting with `-----BEGIN` — plaintext PEM (Node.js / .NET path)
///   - `Setting::String` (other) — base64-encoded PEM or DER (JDBC / ODBC / Go path)
///
/// `private_key_file` is accepted as an alternative: the file is read and returned verbatim.
/// Setting both `private_key` and `private_key_file` is an error.
pub(super) fn read_private_key(settings: &dyn Settings) -> Result<SensitiveString, ConfigError> {
    let has_private_key = settings.get("private_key").is_some();
    let has_private_key_file = settings.get_string("private_key_file").is_some();

    if has_private_key && has_private_key_file {
        return ConflictingParametersSnafu {
            explanation:
                "Both 'private_key' and 'private_key_file' are set. Please provide only one."
                    .to_string(),
        }
        .fail();
    }

    // Bytes (DER from Python)
    if let Some(Setting::Bytes(private_key_bytes)) = settings.get("private_key") {
        return der_to_pem(&private_key_bytes);
    }

    // String: plaintext PEM (starts with `-----BEGIN`), or base64-encoded PEM/DER.
    if let Some(private_key_str) = settings.get_string("private_key") {
        if private_key_str.trim_start().starts_with("-----BEGIN") {
            return Ok(SensitiveString::from(private_key_str));
        }

        let private_key_bytes =
            general_purpose::STANDARD
                .decode(&private_key_str)
                .map_err(|e| {
                    InvalidParameterValueSnafu {
                        parameter: "private_key".to_string(),
                        value: "(redacted)".to_string(),
                        explanation: format!("Could not decode base64 private key: {e}"),
                    }
                    .build()
                })?;

        if private_key_bytes.starts_with(b"-----BEGIN") {
            return String::from_utf8(private_key_bytes)
                .map(SensitiveString::from)
                .map_err(|e| {
                    InvalidParameterValueSnafu {
                        parameter: "private_key".to_string(),
                        value: "(redacted)".to_string(),
                        explanation: format!("Private key is not valid UTF-8: {e}"),
                    }
                    .build()
                });
        }

        return der_to_pem(&private_key_bytes);
    }

    // private_key was set but isn't a String or Bytes (e.g. Int/Double/Bool) — report the
    // actual problem instead of falling through to "missing parameter". Print only the
    // type name, never the value: if a future change to the branches above ever let a
    // String/Bytes payload reach this arm, we must not risk formatting a secret.
    if let Some(other) = settings.get("private_key") {
        let type_name = match other {
            Setting::String(_) => "string",
            Setting::Bytes(_) => "bytes",
            Setting::Int(_) => "int",
            Setting::Double(_) => "double",
            Setting::Bool(_) => "bool",
        };
        return InvalidParameterValueSnafu {
            parameter: "private_key".to_string(),
            value: format!("(type: {type_name})"),
            explanation: "private_key must be a string or bytes value".to_string(),
        }
        .fail();
    }

    // File path
    if let Some(private_key_file) = settings.get_string("private_key_file") {
        // Gate the read on the same file-permission check that protects other
        // credential-bearing config files, honoring the unsafe opt-out.
        let permission_check =
            if settings.get_bool("unsafe_skip_config_file_permissions_check") == Some(true) {
                FilePermissionCheck::UnsafeDisabled
            } else {
                FilePermissionCheck::Enabled
            };

        let path = std::path::Path::new(&private_key_file);
        // The permission gate only guards files that actually exist — the same
        // order `load_toml_file` uses. A missing or inaccessible file must
        // surface the private-key-specific read error below (the driver's
        // documented error for a bad `private_key_file` path), not a generic
        // config-read error raised while stat-ing the file for its mode.
        if path.try_exists().unwrap_or(false) {
            check_file_permissions(path, permission_check)?;
        }

        let private_key = fs::read_to_string(path).map_err(|e| {
            InvalidParameterValueSnafu {
                parameter: "private_key_file".to_string(),
                value: private_key_file,
                explanation: format!("Could not read private key file: {e}"),
            }
            .build()
        })?;
        return Ok(SensitiveString::from(private_key));
    }

    MissingParameterSnafu {
        parameter: "private_key or private_key_file".to_string(),
    }
    .fail()
}

pub(super) fn has_private_key_params(settings: &dyn Settings) -> bool {
    settings.get("private_key").is_some() || settings.get_string("private_key_file").is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::param_store::ParamStore;
    use base64::engine::general_purpose::STANDARD;

    fn settings_with(pairs: &[(&str, Setting)]) -> ParamStore {
        let mut store = ParamStore::with_registry_defaults();
        for (key, value) in pairs {
            store.insert((*key).to_string(), value.clone());
        }
        store
    }

    // --- read_private_key ---

    #[test]
    fn read_private_key_accepts_plaintext_pem_with_leading_whitespace() {
        let pem = "\n  -----BEGIN ENCRYPTED PRIVATE KEY-----\nMIIFake\n-----END ENCRYPTED PRIVATE KEY-----\n";
        let settings = settings_with(&[("private_key", Setting::String(pem.into()))]);
        assert_eq!(read_private_key(&settings).unwrap().reveal(), pem);
    }

    #[test]
    fn read_private_key_accepts_base64_encoded_pem() {
        let pem = "-----BEGIN PRIVATE KEY-----\nMIIFake\n-----END PRIVATE KEY-----\n";
        let b64 = STANDARD.encode(pem.as_bytes());
        let settings = settings_with(&[("private_key", Setting::String(b64))]);
        assert_eq!(read_private_key(&settings).unwrap().reveal(), pem);
    }

    #[test]
    fn read_private_key_rejects_base64_encoded_non_pem_der() {
        // base64(non-PEM bytes) → reaches der_to_pem → OpenSSL rejects malformed DER
        let b64 = STANDARD.encode(b"\x30\x82\x00\x00fake");
        let settings = settings_with(&[("private_key", Setting::String(b64))]);
        let err = read_private_key(&settings).unwrap_err();
        assert!(
            matches!(err, ConfigError::InvalidParameterValue { ref explanation, .. }
                if explanation.contains("Could not parse DER private key")),
            "expected DER parse failure, got: {err}"
        );
    }

    #[test]
    fn read_private_key_bytes_der_reaches_der_to_pem() {
        // Setting::Bytes (Python DER path) → der_to_pem → OpenSSL rejects malformed DER
        let fake_der = b"\x30\x82\x00\x00fake".to_vec();
        let settings = settings_with(&[("private_key", Setting::Bytes(fake_der))]);
        let err = read_private_key(&settings).unwrap_err();
        assert!(
            matches!(err, ConfigError::InvalidParameterValue { ref explanation, .. }
                if explanation.contains("Could not parse DER private key")),
            "expected DER parse failure, got: {err}"
        );
    }

    #[test]
    fn read_private_key_rejects_invalid_base64() {
        let settings =
            settings_with(&[("private_key", Setting::String("not-valid-base64!!!".into()))]);
        let err = read_private_key(&settings).unwrap_err();
        assert!(
            matches!(err, ConfigError::InvalidParameterValue { ref explanation, .. }
                if explanation.contains("Could not decode base64 private key")),
            "got: {err}"
        );
    }

    #[test]
    fn read_private_key_rejects_wrong_type() {
        // private_key set to a non-String/non-Bytes value should report the actual
        // problem, not fall through to "missing parameter".
        let settings = settings_with(&[("private_key", Setting::Int(42))]);
        let err = read_private_key(&settings).unwrap_err();
        assert!(
            matches!(err, ConfigError::InvalidParameterValue { ref explanation, .. }
                if explanation.contains("must be a string or bytes value")),
            "expected wrong-type error, got: {err}"
        );
    }

    #[test]
    fn read_private_key_accepts_bytes_der_round_trip() {
        // Setting::Bytes (Python DER path) with a real key should decode to valid PEM.
        use openssl::rsa::Rsa;
        let rsa = Rsa::generate(2048).expect("generate rsa key");
        let pkey = PKey::from_rsa(rsa).expect("pkey from rsa");
        let der = pkey.private_key_to_der().expect("encode der");

        let settings = settings_with(&[("private_key", Setting::Bytes(der))]);
        let pem = read_private_key(&settings).expect("should decode DER to PEM");
        assert!(pem.reveal().starts_with("-----BEGIN PRIVATE KEY-----"));
        assert!(PKey::private_key_from_pem(pem.reveal().as_bytes()).is_ok());
    }

    #[test]
    fn read_private_key_accepts_base64_der_round_trip() {
        // Setting::String base64(DER) with a real key should decode to valid PEM.
        use openssl::rsa::Rsa;
        let rsa = Rsa::generate(2048).expect("generate rsa key");
        let pkey = PKey::from_rsa(rsa).expect("pkey from rsa");
        let der = pkey.private_key_to_der().expect("encode der");
        let b64 = STANDARD.encode(&der);

        let settings = settings_with(&[("private_key", Setting::String(b64))]);
        let pem = read_private_key(&settings).expect("should decode base64(DER) to PEM");
        assert!(pem.reveal().starts_with("-----BEGIN PRIVATE KEY-----"));
        assert!(PKey::private_key_from_pem(pem.reveal().as_bytes()).is_ok());
    }

    #[test]
    fn read_private_key_rejects_conflict() {
        let settings = settings_with(&[
            (
                "private_key",
                Setting::String(
                    "-----BEGIN PRIVATE KEY-----\nfake\n-----END PRIVATE KEY-----\n".into(),
                ),
            ),
            ("private_key_file", Setting::String("/some/path".into())),
        ]);
        assert!(matches!(
            read_private_key(&settings),
            Err(ConfigError::ConflictingParameters { .. })
        ));
    }

    #[test]
    fn read_private_key_missing_returns_error() {
        let settings = settings_with(&[]);
        assert!(matches!(
            read_private_key(&settings),
            Err(ConfigError::MissingParameter { .. })
        ));
    }

    #[test]
    fn read_private_key_file_missing_returns_read_error() {
        // A nonexistent `private_key_file` must surface the private-key-specific
        // read error, not the permission gate's config-read error raised while
        // stat-ing the file for its mode. Regression for the permission-check-
        // before-read ordering: the JDBC/Python "invalid private key" e2e tests
        // match on the "Could not read private key file" substring.
        let settings = settings_with(&[(
            "private_key_file",
            Setting::String("/nonexistent/definitely_missing_key.p8".into()),
        )]);
        let err = read_private_key(&settings).unwrap_err();
        assert!(
            matches!(err, ConfigError::InvalidParameterValue { ref explanation, .. }
                if explanation.contains("Could not read private key file")),
            "expected private-key read error, got: {err}"
        );
    }

    // --- has_private_key_params ---

    #[test]
    fn has_private_key_params_with_string() {
        let s = settings_with(&[("private_key", Setting::String("any".into()))]);
        assert!(has_private_key_params(&s));
    }

    #[test]
    fn has_private_key_params_with_bytes() {
        let s = settings_with(&[("private_key", Setting::Bytes(vec![1, 2, 3]))]);
        assert!(has_private_key_params(&s));
    }

    #[test]
    fn has_private_key_params_with_file() {
        let s = settings_with(&[("private_key_file", Setting::String("/path/to/key".into()))]);
        assert!(has_private_key_params(&s));
    }

    #[test]
    fn has_private_key_params_with_neither() {
        let s = settings_with(&[]);
        assert!(!has_private_key_params(&s));
    }

    // --- private_key_file permission gate (Unix only) ---

    #[cfg(unix)]
    mod private_key_file_permissions {
        use super::*;
        use std::fs;
        use std::os::unix::fs::PermissionsExt;
        use tempfile::NamedTempFile;

        fn pem_key_content() -> &'static str {
            "-----BEGIN PRIVATE KEY-----\nfake\n-----END PRIVATE KEY-----\n"
        }

        #[test]
        fn should_reject_private_key_file_writable_by_group_or_others() {
            let tmp = NamedTempFile::new().unwrap();
            fs::write(tmp.path(), pem_key_content()).unwrap();
            // Group/other-writable (0o666) is rejected; read-only-permissive
            // modes like 0o644 only warn, so use a writable mode here.
            fs::set_permissions(tmp.path(), fs::Permissions::from_mode(0o666)).unwrap();

            let settings = settings_with(&[(
                "private_key_file",
                Setting::String(tmp.path().to_str().unwrap().into()),
            )]);
            let err = read_private_key(&settings).unwrap_err();
            assert!(
                matches!(err, ConfigError::InsecurePermissions { .. }),
                "Expected InsecurePermissions, got: {err:?}"
            );
        }

        #[test]
        fn should_load_private_key_file_with_restricted_mode() {
            let tmp = NamedTempFile::new().unwrap();
            fs::write(tmp.path(), pem_key_content()).unwrap();
            fs::set_permissions(tmp.path(), fs::Permissions::from_mode(0o600)).unwrap();

            let settings = settings_with(&[(
                "private_key_file",
                Setting::String(tmp.path().to_str().unwrap().into()),
            )]);
            let result = read_private_key(&settings).unwrap();
            assert_eq!(result.reveal(), pem_key_content());
        }

        #[test]
        fn should_skip_permission_check_when_unsafe_opt_out_set() {
            let tmp = NamedTempFile::new().unwrap();
            fs::write(tmp.path(), pem_key_content()).unwrap();
            // 0o666 would be rejected without the opt-out; the bypass lets it load.
            fs::set_permissions(tmp.path(), fs::Permissions::from_mode(0o666)).unwrap();

            let settings = settings_with(&[
                (
                    "private_key_file",
                    Setting::String(tmp.path().to_str().unwrap().into()),
                ),
                (
                    "unsafe_skip_config_file_permissions_check",
                    Setting::Bool(true),
                ),
            ]);
            let result = read_private_key(&settings).unwrap();
            assert_eq!(result.reveal(), pem_key_content());
        }
    }
}
