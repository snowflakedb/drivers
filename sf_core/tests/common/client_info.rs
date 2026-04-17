//! Shared `ClientInfo` builder for integration and e2e tests.
//!
//! Provides a single source of truth for "test-flavored" `ClientInfo` values
//! (insecure TLS, generic identifiers, no OCSP). Override individual fields
//! with struct-update syntax:
//!
//! ```ignore
//! let ci = ClientInfo {
//!     application: "sf_core_test".to_string(),
//!     ..test_client_info()
//! };
//! ```

use sf_core::config::rest_parameters::ClientInfo;
use sf_core::crl::config::CrlConfig;
use sf_core::tls::config::TlsConfig;

/// `ClientInfo` with safe-for-mock-server defaults: insecure TLS so tests can
/// hit local listeners without valid certs, no OCSP, generic `test-os`
/// identifiers. Adding a new field to `ClientInfo` only requires updating
/// this function.
pub fn test_client_info() -> ClientInfo {
    ClientInfo {
        application: "test".to_string(),
        version: "1.0.0".to_string(),
        os: "test-os".to_string(),
        os_version: "1.0".to_string(),
        ocsp_mode: None,
        crl_config: CrlConfig::default(),
        tls_config: TlsConfig::insecure(),
        platforms: Vec::new(),
    }
}
