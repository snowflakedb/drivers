> Derived-from: snowflake-eng/drivers@626637efa4f9ed2168b572d84cfea24009332993 · generated 2026-08-19 · sources: snowflake-eng/drivers@626637efa4f9ed2168b572d84cfea24009332993
> Regenerate when: the TLS-verification-bypass parameters gain a compile-time or build-mode gate that does not exist today

## Do deployments differ in security-relevant ways

No. The TLS-verification-bypass connection parameters (`tls_skip_verify`,
`verify_certificates`, `verify_hostname`, described in `trust-boundaries.md`)
are gated identically in every build — an ordinary runtime connection
parameter, resolved through the same `Settings`-map path as every other
connection parameter, in every build this repository produces.

- Not a compile-time feature: `sf_core/Cargo.toml`'s feature list (`default =
  ["protobuf", "sonic-json"]`, plus `fips`, `vendored-openssl`, `auth_*_e2e`,
  `gcs_e2e`, `perf_timing`, `test-utils`, `cli`) has no TLS-bypass feature, and
  no `cfg(feature = ...)` gate exists around
  `sf_core/src/tls/config.rs#TlsConfig::from_settings` or
  `sf_core/src/tls/client.rs`'s use of it. The `fips` feature changes only the
  linked TLS crypto backend — the test suite is identical with and without it
  (`.github/workflows/test-rust-core.yml:417-419`).
- The runtime toggle is reachable outside tests: the standalone `tls_client`
  diagnostic binary (`sf_core/src/bin/tls_client.rs:166-179`) exposes
  `--no-verify-hostname`/`--no-verify-certs`/`--insecure` CLI flags that set
  the same `TlsConfig` fields at runtime, using the same
  `create_tls_client_with_config` function production connections use.

So the only variable is what value the calling application supplies at
connect time — there is no per-environment or per-build divergence to record.
