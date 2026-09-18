// `query_logging.rs` builds a raw `reqwest::Client`, and this target does not
// include `common/mod.rs`, so it needs the provider initializer of its own.
#[path = "common/crypto_provider.rs"]
mod crypto_provider;

#[path = "integration/logging/query_logging.rs"]
mod query_logging;
