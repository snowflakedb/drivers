use std::path::PathBuf;

use crate::token_cache::TokenCache;

/// Paths to system files read at runtime (tokens, OS metadata, etc.).
///
/// Production code gets these via [`AuthContext::default()`] which resolves
/// the real platform paths. Tests construct the struct directly, pointing
/// fields at temp files.
pub struct RuntimePaths {
    pub spcs_token_file: PathBuf,
}

impl Default for RuntimePaths {
    fn default() -> Self {
        Self {
            spcs_token_file: PathBuf::from("/snowflake/session/spcs_token"),
        }
    }
}

/// Runtime/environmental context for the authentication flow.
///
/// Bundles dependencies that are not login credentials but are needed
/// during authentication (file-system paths, token caches, etc.).
/// The [`Default`] impl uses real platform paths and no token cache.
/// Tests construct the struct directly to inject fakes.
#[derive(Default)]
pub struct AuthContext<'a> {
    pub runtime_paths: RuntimePaths,
    pub token_cache: Option<&'a dyn TokenCache>,
}
