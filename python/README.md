# PEP 249 Database API 2.0 Implementation

A Python library that implements [PEP 249 (Python Database API Specification 2.0)](https://peps.python.org/pep-0249/) with empty interface implementations. This library provides a complete skeleton implementation that follows the PEP 249 specification, making it an ideal starting point for creating new database drivers or for testing database API compliance.

## Development
To build core library for local development run:
```bash
make build-core
```

## Testing

### Quick Start

```bash
cd pep249_dbapi/

# Install dependencies and run all tests
make setup
make test
```

### Detailed Commands

```bash
# Setup environment (installs uv, syncs dependencies)
make setup

# Run all tests (unit, integration, e2e) - recommended
make test

# Run specific test types
make test tests/unit/          # Unit tests only
make test tests/integ/         # Integration tests only  
make test tests/e2e/           # End-to-end tests only

# Run with specific Python version
make test PYTHON_VERSION=3.12

# Run specific tests with pytest arguments
make test -- -k test_connection --maxfail=1
make test PYTEST_ARGS="-k test_connection --maxfail=1"

# Fast local testing (skip tox isolation)
make test-local

# Sequential testing (for debugging race conditions)
make test-local-sequential

# Compare universal vs reference driver
make compare-local
make compare-local REFERENCE_DRIVER_VERSION=3.18.0
```

### Requirements
- Python 3.9+
- Rust core library: `../target/debug/libsf_core.{so,dylib}` (auto-built if missing)
- Credentials: `../parameters.json` (see main [README.md](../README.md) for setup instructions)

## References

- [PEP 249 - Python Database API Specification v2.0](https://peps.python.org/pep-0249/)
- [Python Database API Specification v2.0](https://www.python.org/dev/peps/pep-0249/) 