# SF Core - Rust Core Library

The core Rust library that powers the universal driver. This library provides the fundamental database driver functionality including connection management, query execution, authentication, and data processing.

## Testing

### Prerequisites

See [Prerequisites](../README.md#prerequisites) section in the top-level README for required setup steps.

### Running Tests

```bash
# Set required environment variable
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

### Requirements

- `PARAMETER_PATH` environment variable pointing to `parameters.json`

