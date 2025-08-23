# Universal Driver Testing

**Run all commands from the `pep249_dbapi/` directory.**

## Setup

```bash
cd pep249_dbapi/
make setup  # Install deps, create venv
```

Requires:
- Rust core built: `../target/debug/libsf_core.{so,dylib}`
- Credentials: `../parameters.json`

## Quick Testing

### Run all tests (preferred)
Tox creates own isolated venv - not dependent on local env
```bash
make tox
```

### Run specific test with different Python version
```bash
make tox tests/integ/test_connection.py PYTHON_VERSION=3.12
```

### Compare both drivers
Runs integ tests on both and returns report
```bash
make compare-local
```

## Additional testing options

### Basic Commands
```bash
make test                    # All tests (universal driver) - local uv env
make test-integ-local-tox    # Integration tests only
make test-reference-local    # Reference driver tests
```

### Test Arguments
Pass pytest arguments two ways:
```bash
make tox PYTEST_ARGS="-k test_connection"  # Keyword way
make tox -k test_connection                 # Direct way (trailing args)
```

### Examples
```bash
make tox tests/integ/test_connection.py                    # Single file
make tox -k test_connection                                # Pattern match
make tox -m "not slow"                                     # Skip tests marked as @pytest.mark.slow (or any other mark)
make tox --maxfail=1 -vv                                   # Stop on first failure
make tox PYTHON_VERSION=3.12                              # Different Python version
make tox tests/unit/ PYTHON_VERSION=3.12 # Unit tests + version
```

## Test Structure

```
tests/
├── unit/    # Fast tests, no DB connection
└── integ/   # Requires Snowflake connection
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

## Comparison

`make compare-local` runs integration tests on both drivers and compares results. The comparison automatically filters to only compare integration tests for fair comparison (universal runs unit+integ, reference only integ).

Report sections:
- **Regressions from passing**: Reference passed, universal failed (we broke something)
- **Regressions from failing**: Reference failed, universal passed (behavioral differences)
- **Both failing**: Expected differences
- **Skipped differences**: Different skip behavior

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
- `REFERENCE_DRIVER_VERSION`: Reference driver version (default: 3.17.2)

## Debugging

```bash
make test -vv        # Verbose output
make test --pdb      # Debug on failure
make test --tb=long  # Full traceback
```

## CI Workflow

GitHub Actions: Build Rust core once → Test universal (all tests, Python 3.9-3.13) → Test reference (integ only, Python 3.13) → Compare (integ only for fair comparison)

## Make Targets

| Command | Description |
|---------|-------------|
| `setup` | Install dependencies |
| `test` / `tox` | All tests (aliases) |
| `test-integ-local-tox` | Integration tests only |
| `test-reference-local` | Reference driver tests |
| `compare-local` | Test both + compare |
| `clean-reports` | Remove report files |

All targets accept `PYTHON_VERSION` and `PYTEST_ARGS`.

## Troubleshooting

**Missing core library**: `cd ../sf_core && cargo build`

**Connection issues**: Check `parameters.json` and network access

**Test failures**: Use `-vv` for details, check if it's expected behavioral difference

**Tox environment issues**: Clean with `rm -rf .tox` and retry