# Universal Driver Testing

Tests the universal driver (Rust-backed) against the reference Snowflake connector to catch behavioral differences.

## Setup

```bash
make setup  # Install deps, create venv
```

Requires:
- Rust core built: `../target/debug/libsf_core.{so,dylib}`
- Credentials: `../parameters.json`

## Running Tests

```bash
make test-local              # All tests (universal driver)
make test-unit-local         # Unit tests only
make test-integ-local        # Integration tests (universal)
make test-reference-local    # Integration tests (reference)
make compare-local           # Run both + compare
```

## Test Structure

```
tests/
├── unit/    # Fast tests, no DB connection
└── integ/   # Requires Snowflake connection
```

## Specific Tests

```bash
# Single test file
make test-local PYTEST_ARGS="tests/integ/test_connection.py"

# Single test method  
make test-local PYTEST_ARGS="tests/integ/test_connection.py::TestConnectionMethods::test_close_connection"

# Pattern matching
make test-local PYTEST_ARGS="-k test_connection"

# Skip slow tests
make test-local PYTEST_ARGS="-m 'not slow'"

# Stop on first failure
make test-local PYTEST_ARGS="--maxfail=1 -vv"

# Different Python version
make test-local PYTHON_VERSION=3.12
```

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
        # Use different database
        pass
```

## Comparison Reports

```bash
make compare-local  # Runs both drivers, shows differences
```

Report sections:
- **Regressions**: Reference passes, universal fails
- **Breaking changes**: Reference fails, universal passes  
- **Both failing**: Expected differences
- **Skip differences**: Different skip behavior

## Adding Tests

**Unit tests** (`tests/unit/`): No database required
```python
def test_module_constant():
    assert pep249_dbapi.apilevel == "2.0"
```

**Integration tests** (`tests/integ/`): Need database connection
```python
class TestNewFeature:
    def test_method(self, connection):
        cursor = connection.cursor()
        # Test actual behavior
        
    @pytest.mark.slow
    def test_large_data(self, cursor):
        # Mark slow tests
        pass
```

## Environment Variables

Auto-detected locally, set manually in CI:
- `CORE_PATH`: Path to `libsf_core.{so,dylib}`
- `PARAMETER_PATH`: Path to `parameters.json`
- `PYTHON_VERSION`: Python version (default: current)
- `PYTEST_ARGS`: Extra pytest arguments
- `REFERENCE_DRIVER_VERSION`: Reference driver version

## Debugging

```bash
# Verbose output
make test-local PYTEST_ARGS="-vv"

# Debug on failure
make test-local PYTEST_ARGS="--pdb"

# Full traceback
make test-local PYTEST_ARGS="--tb=long"
```

## CI

Runs on GitHub Actions:
1. Build Rust core (once)
2. Test universal driver (Python 3.9-3.13)
3. Test reference driver (Python 3.13)
4. Compare results

## Make Targets

| Command | Description |
|---------|-------------|
| `setup` | Install dependencies |
| `test-local` | All tests (universal) |
| `test-local-tox` | Integration tests with tox |
| `test-unit-local` | Unit tests only |
| `test-integ-local` | Integration tests (universal) |
| `test-reference-local` | Integration tests (reference) |
| `compare-local` | Test both + compare |
| `clean-reports` | Remove report files |

All targets accept `PYTHON_VERSION` and `PYTEST_ARGS`.

## Troubleshooting

**Missing core library**: `cd ../sf_core && cargo build`

**Connection issues**: Check `parameters.json` and network access

**Test failures**: Use `-vv` for details, check if it's expected difference