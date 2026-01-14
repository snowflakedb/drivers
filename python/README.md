# PEP 249 Database API 2.0 Implementation

A Python library that implements [PEP 249 (Python Database API Specification 2.0)](https://peps.python.org/pep-0249/) with empty interface implementations. This library provides a complete skeleton implementation that follows the PEP 249 specification, making it an ideal starting point for creating new database drivers or for testing database API compliance.

## Development

### Prerequisites
- Python 3.9+
- [uv](https://docs.astral.sh/uv/) package manager
- [Hatch](https://hatch.pypa.io/) build tool
- Rust toolchain (for building core library)
- Credentials: `../parameters.json` (see main [README.md](../README.md) for setup instructions)

### Setup

Install uv and Hatch:
```bash
# Install uv
curl -LsSf https://astral.sh/uv/install.sh | sh

# Install Hatch
uv tool install hatch
```

Build the Rust core library:
```bash
hatch run build-core
```

### Environment Variables

**IMPORTANT:** Set up required environment variables before running tests:

```bash
# Easy way (recommended) - use the setup script:
source scripts/setup_env.sh

# Or manually with auto-detection:
eval $(python scripts/detect_core_path.py --export)

# Or set manually:
export PARAMETER_PATH="$(pwd)/../parameters.json"
```

You can verify the detected path with:
```bash
hatch run show-paths
```

**Note:** Unlike the old Makefile which auto-detected paths, you must explicitly set `PARAMETER_PATH` before running tests. If not set, it will be an empty string, which will cause test failures.

## Testing

See README in `tests/` directory

### Quick Start

```bash
# Run all tests (unit, integration, e2e)
hatch run test:all

# Run with specific Python version
hatch run test.py3.12:all
```

### Detailed Commands

```bash
# Run specific test types
hatch run test:unit              # Unit tests only
hatch run test:integ             # Integration tests only
hatch run test:e2e               # End-to-end tests only
hatch run test:all               # All tests

# Run with coverage
hatch run test:all-cov

# Run with specific Python version
hatch run test.py3.9:all
hatch run test.py3.10:all
hatch run test.py3.11:all
hatch run test.py3.12:all
hatch run test.py3.13:all

# Pass additional pytest arguments
hatch run test:all -k test_connection --maxfail=1

# Run reference connector tests
REFERENCE_DRIVER_VERSION=3.17.2 hatch run reference:run

# Lint and type checking
hatch run lint:run
hatch run type:run
```

## References

- [PEP 249 - Python Database API Specification v2.0](https://peps.python.org/pep-0249/)
- [Python Database API Specification v2.0](https://www.python.org/dev/peps/pep-0249/) 