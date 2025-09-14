#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CertRevocationCheckMode {
    Disabled,
    Advisory,
    Enabled,
}

#[derive(Debug, Clone)]
pub struct CrlConfig {
    pub enabled: bool,
    pub mode: CertRevocationCheckMode,
    pub outcome_cache_capacity: usize,
}

impl Default for CrlConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: CertRevocationCheckMode::Disabled,
            outcome_cache_capacity: 10_000,
        }
    }
}
