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

## All Available Commands

### Basic Testing
```bash
make test                    # All tests (universal driver) - short alias
make test-local              # All tests (universal driver) 
make tox                     # All tests with tox (isolated env) - short alias
make test-local-tox          # All tests with tox (isolated env)
```

### Integration Testing
```bash
make test-integ-local-tox    # Integration tests only with tox
make test-reference-local    # Integration tests (reference driver)
make test-reference-local-tox # Integration tests (reference) with tox
```

### Comparison
```bash
make compare-local           # Run integration tests on both + compare
```

### Utility
```bash
make setup                   # Install dependencies
make clean-reports           # Remove report files
```

## Test Structure

```
tests/
├── unit/    # Fast tests, no DB connection
└── integ/   # Requires Snowflake connection
```

## Passing Arguments

You can pass pytest arguments in two ways:

```bash
# Key-word way
make test PYTEST_ARGS="-k test_connection"

# Direct way (trailing arguments)
make test -k test_connection
make test tests/integ/test_connection.py
make test --maxfail=1 -vv
make test -m "not slow"
```

## Specific Tests

```bash
# Single test file
make test tests/integ/test_connection.py

# Single test method  
make test tests/integ/test_connection.py::TestConnectionMethods::test_close_connection

# Pattern matching
make test -k test_connection

# Skip tests marked using @pytest.mark.slow
make test -m "not slow"

# Stop on first failure
make test --maxfail=1 -vv

# Different Python version
make test PYTHON_VERSION=3.12
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
make compare-local  # Runs integration tests on both drivers, shows differences
```

The comparison automatically filters to only compare integration tests since:
- Universal driver runs both unit + integration tests
- Reference driver only runs integration tests  
- Comparison script filters universal results to `tests/integ/*` for fair comparison

Report sections:
- **Regressions from passing**: Reference passed, universal failed (bad - we broke something)
- **Regressions from failing**: Reference failed, universal passed (behavioral differences)
- **Both failing**: Expected differences
- **Skipped only on universal/reference**: Different skip behavior

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
# Verbose output
make test -vv

# Debug on failure
make test --pdb

# Full traceback
make test --tb=long
```

## CI Workflow

GitHub Actions runs:
1. **Build Rust core** (once, shared across jobs)
2. **Test universal driver** (Python 3.9-3.13, all tests)
3. **Test reference driver** (Python 3.13 only, integration tests)
4. **Compare results** (integration tests only for fair comparison)

The CI automatically:
- Runs comprehensive tests on universal driver
- Compares only integration tests (both drivers can run these)
- Generates comparison reports showing behavioral differences

## Make Targets

| Command | Description |
|---------|-------------|
| `setup` | Install dependencies |
| `test` | All tests (universal) - short alias |
| `test-local` | All tests (universal) |
| `tox` | All tests with tox - short alias |
| `test-local-tox` | All tests with tox (isolated env) |
| `test-integ-local-tox` | Integration tests only with tox |
| `test-reference-local` | Integration tests (reference) |
| `test-reference-local-tox` | Integration tests (reference) with tox |
| `compare-local` | Run integration tests on both + compare |
| `clean-reports` | Remove report files |

All targets accept `PYTHON_VERSION` and `PYTEST_ARGS`.

## Troubleshooting

**Missing core library**: `cd ../sf_core && cargo build`

**Connection issues**: Check `parameters.json` and network access

**Test failures**: Use `-vv` for details, check if it's expected behavioral difference

**Tox environment issues**: Clean with `rm -rf .tox` and retry