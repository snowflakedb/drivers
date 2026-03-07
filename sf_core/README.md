# SF Core - Rust Core Library

The core Rust library that powers the universal driver. This library provides the fundamental database driver functionality including connection management, query execution, authentication, and data processing.

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

### Requirements

- `PARAMETER_PATH` environment variable pointing to `parameters.json`
- **For VPN tests:** VPN connection to Snowflake network

