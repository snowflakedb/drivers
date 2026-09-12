extern crate tracing;
extern crate tracing_subscriber;

pub mod apis;
pub mod diagnostic;
pub mod env_vars;

pub mod arrow_utils;
mod auth;
pub mod c_api;
pub mod chunks;
mod compression;
mod compression_types;
pub mod config;
pub mod crl;
mod fs_lock;
pub mod xp_backend;
// Public for integration tests; only `types` and specific transfer functions are re-exported.
pub mod file_manager;
pub mod fs_adapter;
pub mod handle_manager;
pub mod http;
pub mod logging;
pub mod perf_timing;
pub mod query_types;
pub mod refresh;
pub mod rest;
pub mod sensitive;
pub mod stage_binding;
pub mod telemetry;
pub mod tls;
pub mod token_cache;
pub mod utils;

#[cfg(feature = "protobuf")]
pub mod protobuf;

// Unit tests across this crate build ad-hoc `reqwest::Client`s (wiremock
// servers, telemetry fakes) without going through the driver's TLS factories.
// The dev reqwest dependency uses the `-no-provider` rustls features (see
// Cargo.toml), so a client built before any provider install panics with
// "No provider set"; install the process default up front so tests are not
// order-dependent. Production builds have no such initializer -- they rely on
// `tls::ensure_crypto_provider()` at the client-construction chokepoints.
#[cfg(test)]
#[ctor::ctor(unsafe)]
fn install_default_crypto_provider_for_tests() {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
}
