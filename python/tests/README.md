# Universal Driver Testing

## Setup

### Prerequisites
- Python 3.9+
- [uv](https://docs.astral.sh/uv/) package manager
- [Hatch](https://hatch.pypa.io/) build tool
- Rust core library: `../target/debug/libsf_core.{so,dylib}` (built with `hatch run build-core`)
- Credentials: `../parameters.json` (for integration tests)

### Installation

```bash
# Install uv
curl -LsSf https://astral.sh/uv/install.sh | sh

# Install Hatch
uv tool install hatch

# Build Rust core library
hatch run build-core

# Set up environment variables (REQUIRED for tests)
source scripts/setup_env.sh
```

### Environment Variables

**CRITICAL:** Tests require `PARAMETER_PATH` to be set explicitly.

#### Quick Setup (Recommended)

```bash
# One-liner - sets up everything and validates paths:
source scripts/setup_env.sh
```

#### Manual Setup

```bash
# Auto-detect path:
eval $(python scripts/detect_core_path.py --export)

# Or set manually:
export PARAMETER_PATH="$(pwd)/../parameters.json"

# Verify path is set correctly:
hatch run show-paths
echo $PARAMETER_PATH
```

## Quick Start

### Run all tests (recommended)
```bash
hatch run test:all  # All tests with parallel execution
```

### Run specific tests
```bash
hatch run test:all -k test_connection --maxfail=1
```

### Test with different Python version
```bash
hatch run test.py3.12:all
```

### Compare universal vs reference drivers
```bash
# Run universal tests
hatch run test:all --json-report --json-report-file=reports/universal.json

# Run reference tests
REFERENCE_DRIVER_VERSION=3.17.2 hatch run reference:run --json-report --json-report-file=reports/reference.json

# Compare results
hatch run ci:compare --py 3.13 --os ubuntu-latest --universal reports/universal.json --reference reports/reference.json --fail-on-regressions 0
```

## Testing Commands

### Test Environments

| Command | Description                                  | Use Case                                                                          |
|---------|----------------------------------------------|-----------------------------------------------------------------------------------|
| `hatch run test:all` | All tests (unit + integ + e2e)               | Full testing suite with proper isolation                                          |
| `hatch run test:unit` | Unit tests only                              | Fast tests without external dependencies                                          |
| `hatch run test:integ` | Integration tests only                       | Tests requiring database connection                                               |
| `hatch run test:e2e` | End-to-end tests only                        | Complete workflow tests                                                           |
| `hatch run test:all-cov` | All tests with coverage                      | Generate coverage reports                                                         |
| `hatch run reference:run` | Reference driver testing                     | Testing with official snowflake-connector-python                                  |

### Matrix Testing

Run tests across all supported Python versions:
```bash
hatch run test.py3.9:all
hatch run test.py3.10:all
hatch run test.py3.11:all
hatch run test.py3.12:all
hatch run test.py3.13:all
```

### Code Quality

```bash
hatch run precommit:check    # Run all checks (format, lint, type)
hatch run precommit:fix      # Auto-fix formatting and linting
```

### Build

```bash
hatch build              # Build wheel and sdist
hatch run build-core     # Build Rust core library
```

## Common Pytest Options

Pass additional pytest arguments directly:
```bash
hatch run test:all --maxfail=1           # Stop on first failure
hatch run test:all -vv                   # Extra verbose
hatch run test:all -m 'not slow'         # Skip slow tests
hatch run test:all -n auto               # Parallel execution (auto detect cores)
```

## Test Markers
- `@pytest.mark.skip_universal(reason="...")` - Skip on universal driver
- `@pytest.mark.skip_reference(reason="...")` - Skip on reference driver

## Configuration

### Connection Parameters (`../parameters.json`)

```json
{
  "testconnection": {
    "SNOWFLAKE_TEST_ACCOUNT": "your-account",
    "SNOWFLAKE_TEST_USER": "username",
    "SNOWFLAKE_TEST_PASSWORD": "password",
    "SNOWFLAKE_TEST_DATABASE": "database",
    "SNOWFLAKE_TEST_SCHEMA": "schema",
    "SNOWFLAKE_TEST_WAREHOUSE": "warehouse",
    "SNOWFLAKE_TEST_ROLE": "role"
  }
}
```

### Override Parameters in Tests

```python
def test_custom_db(connection_factory):
    with connection_factory(database="test_db") as conn:
        # Use different database for this test
        pass
```

## Environment Variables

### Auto-detected (Local Development)
- `CORE_PATH`: Auto-detects `../target/debug/libsf_core.{so,dylib}` based on platform
- `PARAMETER_PATH`: Auto-detects `../parameters.json`

### Configurable
- `REFERENCE_DRIVER_VERSION`: Reference driver version (default: `3.17.2`)
- `CORE_PATH`: Override core library path
- `PARAMETER_PATH`: Override parameters file path

