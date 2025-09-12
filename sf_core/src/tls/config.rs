use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct TlsConfig {
    pub custom_root_store_path: Option<PathBuf>,
    pub verify_hostname: bool,
    pub verify_certificates: bool,
}

impl TlsConfig {
    pub fn insecure() -> Self {
        Self {
            custom_root_store_path: None,
            verify_hostname: false,
            verify_certificates: false,
        }
    }
}
