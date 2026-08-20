# SF Core - Rust Core Library

The core Rust library that powers the drivers based on unified architecture. This library provides the fundamental database driver functionality including connection management, query execution, authentication, and data processing.

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

