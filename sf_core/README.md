# sf_core

`sf_core` is the Rust library behind the Universal Core. It provides connection
management, query execution, authentication, and data processing for Snowflake
drivers that share that library.

## FIPS build

Enable the `fips` Cargo feature to select AWS-LC-FIPS for rustls and application
cryptography that uses AWS-LC:

```bash
cargo build --package sf_core --features fips
```

This feature is not a validation or whole-driver FIPS compliance claim.
SDK-managed SigV4 signing still uses RustCrypto outside the module, and
certificate and Security Policy applicability for the pinned AWS-LC FIPS
version still require compliance confirmation.

## TLS status

`sf_core::tls::tls_status()` and the handle-free `DriverGetTlsStatus`
protobuf RPC are available in both standard and `fips` builds. They report
two different facts: `tls_provider_is_fips` is rustls's FIPS verdict for the
linked TLS provider, and `fips_build_enabled` says whether this build
enabled the Cargo feature. A standard build normally returns `false` for
both; a correctly configured `fips` build returns `true` for both. A
disagreement indicates that the build flag and provider runtime state differ.

These fields are **not** a claim that every TLS client in the process uses that
provider, that non-TLS cryptography or the driver artifact is FIPS compliant,
or that a particular module version has a validation certificate. They can
be read before opening a connection and do not make a network request.

Wrappers expose the same two facts without a connection:

| Driver | API |
| --- | --- |
| Python | `snowflake.connector.get_tls_status()` |
| JDBC | `new SnowflakeDriver().getTlsStatus()` |
| ODBC | `SFGetTlsStatus(&status)` from `sf_odbc.h` (direct library export, not a driver-manager call) |
| Node.js | `getTlsStatus()` from `snowflake-sdk` |
| .NET | `new SnowflakeDbDriver().GetTlsStatus()` |

## Testing

### Prerequisites

See [Prerequisites](../README.md#prerequisites) section in the top-level README for required setup steps.
Note: some `sf_core` integration tests use Wiremock (standalone JAR) and require **Java** installed on the host.

### Running Tests

```bash
export PARAMETER_PATH=$(pwd)/parameters.json

# Run all tests
cargo test --package sf_core

# Run specific test files
cargo test --package sf_core parameter_bind_tests
cargo test --package sf_core put_get_simple_tests

# Run with output
cargo test --package sf_core -- --nocapture

# Run integration tests only
cargo test --package sf_core --test integration_tests

# Run with coverage
cargo install cargo-llvm-cov
cargo llvm-cov --package sf_core --output-path ./lcov.info
```

### VPN-Required Tests

Some E2E tests require VPN access to Snowflake preprod accounts and are ignored by default. These tests should be prefixed with `vpn_`.

```bash
# Run only VPN-required tests (requires VPN connection)
cargo test -- --ignored vpn_

# Run all tests including VPN-required ones
cargo test -- --include-ignored
```

**Note:** VPN tests are skipped in GitHub Actions CI (no VPN access). Run them on Jenkins or locally with VPN.

### Flaky Tests

Tests that fail intermittently (e.g., due to CI runner timing, server-side propagation delays, or OS credential manager races) are marked with the `flaky_` prefix and `#[ignore]` (or platform-conditional `#[cfg_attr(target_os = "...", ignore)]`).

```rust
// Flaky everywhere
#[test]
#[ignore]
fn flaky_example_test() { ... }

// Flaky only on Windows
#[test]
#[cfg_attr(target_os = "windows", ignore)]
fn flaky_keyring_race_condition() { ... }
```

CI runs flaky tests in a separate non-blocking step for visibility:

```bash
# Run only flaky tests (non-blocking in CI)
cargo test -- --ignored flaky_
```

**Naming conventions for `#[ignore]` tests:**
- `flaky_` prefix → Intermittently failing (non-blocking CI step)
- `vpn_` prefix → Requires VPN access (Jenkins only)
- No prefix → Manual/slow tests (never run in CI)

### Requirements

- `PARAMETER_PATH` environment variable pointing to `parameters.json`
- **For VPN tests:** VPN connection to Snowflake network

