# Universal Driver Testing

**Run all commands from the `pep249_dbapi/` directory.**

## Setup

```bash
cd pep249_dbapi/
make setup  # Install uv, sync dependencies, create reports directory
```

**Requirements:**
- Rust core library: `../target/debug/libsf_core.{so,dylib}` (auto-built if missing)
- Credentials: `../parameters.json` (for integration tests)
- Python 3.9+

## Quick Start

### Run all tests (recommended)
```bash
make test  # Tox-based testing with reports and parallel execution
```

### Run specific tests
Pytest args can be appended after '--' separator as in key-word way (_PYTEST_ARGS_).
```bash
make test -- -k test_connection --maxfail=1
make test PYTEST_ARGS="-k test_connection --maxfail=1"
```

### Test with different Python version
```bash
make test PYTHON_VERSION=3.12
```

### Compare universal vs reference drivers
```bash
make compare-local  # Runs both drivers and compares results
make compare-local REFERENCE_DRIVER_VERSION=3.18.0  # Use specific reference version
```

## Testing Commands

### Local Development Commands

| Command | Description                                  | Use Case                                                                          |
|---------|----------------------------------------------|-----------------------------------------------------------------------------------|
| `make test` | Tox with reports (main)                      | Full testing with proper isolation                                                |
| `make test-local` | Direct pytest (fast)                         | Testing using local environment - e.g. using some specific venv setup             |
| `make test-local-sequential` | Direct pytest (no parallel)                  | Debugging test interactions - searching for race conditions in tests              |
| `make test-local-tox-sequential` | Tox without parallel                         | Debugging in isolated environment - searching for race conditions in tests        
| `make test-integ-local-tox` | Integration tests only                       | Used mainly in `make compare-local`. Can be replaced with `make test tests/integ` |
| `make test-reference-local` | Reference driver testing (integration tests) | Testing whether new changes introduced regression / BCRs                          |

### Generic Runners

| Command | Description | Example                                                        |
|---------|-------------|----------------------------------------------------------------|
| `make run-with-setup` | Run any command with environment | `make run-with-setup echo ${DETECTED_CORE_PATH}`              |
| `make run-with-uv` | Run uv commands with environment | `make run-with-uv pytest tests/unit/test_module.py -- -n auto` |
| `make run-with-tox` | Run specific tox environments | `make run-with-tox py311-unit`                                 |

### CI Commands

| Command | Description | When to Use |
|---------|-------------|-------------|
| `make ci-test-all` | Full CI testing with XML reports | GitHub Actions |
| `make ci-test-integ-reference` | Reference driver CI testing | Comparison baseline |
| `make ci-compare-artifacts` | Compare downloaded CI reports | CI comparison step |



### Common Pytest Options
```bash
# Execution control
PYTEST_ARGS="--maxfail=1"                 # Stop on first failure
# Output control
PYTEST_ARGS="-vv"                         # Extra verbose
# Markers
PYTEST_ARGS="-m 'not slow'"               # Skip slow tests
```

### Test Markers
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

## Comparison

`make compare-local` runs integration tests on both drivers and compares results. The comparison automatically filters to only compare integration tests for fair comparison (universal runs unit+integ, reference only integ).

Report sections:
- **Regressions from passing**: Reference passed, universal failed (we do not support something yet)
- **Regressions from failing**: Reference failed, universal passed (behavioral differences - may require @pytest.mark.skipreference)
- **Both failing**: Not supported in any
- **Skipped differences**: Different skip behavior

## Environment Variables

### Auto-detected (Local Development)
- `CORE_PATH`: Auto-detects `../target/debug/libsf_core.{so,dylib}`
- `PARAMETER_PATH`: Auto-detects `../parameters.json`
- `PYTHON_VERSION`: Auto-detects current Python version

### Configurable
- `PYTHON_VERSION`: Override Python version (e.g., `3.13`, `3.11`)
- `PYTEST_ARGS`: Additional pytest arguments
- `REFERENCE_DRIVER_VERSION`: Reference driver version (default: `3.17.2`)
- `REPORTS_DIR`: Report output directory (default: `reports`)
- `FAIL_ON_REGRESSIONS`: Fail comparison on differences in passing tests between universal and reference driver (default: `0`)

### CI-specific
- `CORE_PATH`: Must be set explicitly in CI
- `UNIVERSAL_TEST_REPORTS_DIR`: Path to universal driver reports
- `REFERENCE_TEST_REPORTS_DIR`: Path to reference driver reports

