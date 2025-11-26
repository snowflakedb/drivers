# PEP 249 Database API 2.0 Implementation

A Python library that implements [PEP 249 (Python Database API Specification 2.0)](https://peps.python.org/pep-0249/) with empty interface implementations. This library provides a complete skeleton implementation that follows the PEP 249 specification, making it an ideal starting point for creating new database drivers or for testing database API compliance.

## Development

### Building the Package

This project uses [Hatch](https://hatch.pypa.io/) as the build backend with [uv](https://github.com/astral-sh/uv) for fast dependency management.

### Building the Core Library

The Rust core library is now built automatically when you build the Python package. However, you can also build it explicitly:

```bash
hatch dev:build-core
```

## Testing

### Quick Start

```bash
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

# Run with specific Python version
make test PYTHON_VERSION=3.12

# Run specific tests with pytest arguments
make test PYTEST_ARGS="-k test_connection --maxfail=1"

# Fast local testing (using uv directly, no isolation)
make test-local

# Sequential testing (for debugging race conditions)
make test-local-sequential

# Compare universal vs reference driver
make compare-local
make compare-local REFERENCE_DRIVER_VERSION=3.18.0
```

### Using Hatch Directly

```bash
# Show available environments
hatch env show

# Run tests in specific environment with Python version
hatch run py3.11:all      # All tests
hatch run py3.11:unit     # Unit tests only
hatch run py3.11:integ    # Integration tests only
hatch run py3.11:e2e      # E2E tests only

# Run tests across all Python versions
hatch run all:all

# Run linting and formatting
hatch run lint:style        # flake8 style check
hatch run lint:format       # auto-format with black
hatch run lint:format-check # check formatting without changing
hatch run lint:typing       # mypy type checking
hatch run lint:all          # run all lint checks

# Reference connector tests
hatch run reference.py3.11:test
```

### Requirements
- Python 3.10+
- Rust core library: `../target/debug/libsf_core.{so,dylib}` (auto-built if missing)
- Credentials: `../parameters.json` (see main [README.md](../README.md) for setup instructions)

## References

- [PEP 249 - Python Database API Specification v2.0](https://peps.python.org/pep-0249/)
- [Python Database API Specification v2.0](https://www.python.org/dev/peps/pep-0249/) 