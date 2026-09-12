// Installs the process-default rustls provider for a test binary.
//
// Why this exists: the dev `reqwest` dependency uses the `-no-provider` rustls
// features (see sf_core/Cargo.toml), matching the production selection so tests
// exercise the same wiring. The consequence is that a raw
// `reqwest::Client::new()` panics with "No provider set" unless a provider was
// installed first, and without this initializer whichever test happened to run
// first would decide whether that happens.
//
// The rule for adding it: every `[[test]]` target is a separate binary, so a
// ctor only runs for the binary it is compiled into. Any target that can reach
// a raw `reqwest::Client` -- directly or through a `#[path]`-included file --
// must pull this module in. Targets that only touch the driver API do not need
// it, because production code calls `tls::ensure_crypto_provider()` itself.
//
// Note the limit of this safety net: because it runs before any test code, a
// production path that forgets `ensure_crypto_provider()` will generally NOT be
// caught by tests. The mechanical guard for that is the planned cargo-deny
// crypto denylist.
#[ctor::ctor(unsafe)]
fn install_default_crypto_provider() {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
}
