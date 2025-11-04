# Performance Testing Framework

## Table of Contents

- [Running Tests](#running-tests)
- [Adding New Tests](#adding-new-tests)
- [Architecture](#architecture)
- [Driver Containers](#driver-containers)
- [Test Structure](#test-structure)
- [Results](#results)
- [Docker Builds Approach](#docker-builds-approach)

---

## Running Tests

### Prerequisites

1. **Docker**: Required for building and running driver containers
2. **Python 3.8+**: For the test runner
3. **Hatch**: Python project manager (install: `pip install hatch`)
4. **GPG**: Required to decrypt test credentials

#### Setup Steps

1. **Decrypt secrets** (required for local testing):
   ```bash
   # From repository root
   ./scripts/decode_secrets.sh
   ```
   

2. **Install Hatch**:
   ```bash
   cd tests/performance
   pip install hatch
   ```

###  Building Driver Images

   Build all drivers:
   ```bash
   hatch run build
   ```

   Or build individually:
   ```bash
   hatch run build-python
   hatch run build-core
   hatch run build-odbc
   ```

#### Platform Architecture

The build system automatically detects your platform architecture and builds appropriate Docker images:

The platform is auto-detected using `detect_platform.sh` based on `uname -m`. You can override this by setting the `BUILDPLATFORM` environment variable:

```bash
BUILDPLATFORM=linux/amd64 hatch run build
```

### Running Tests

#### Local Testing (No Benchstore Upload)

```bash
hatch run core-local
hatch run python-universal-local
hatch run python-old-local
hatch run python-both-local
hatch run odbc-universal-local
hatch run odbc-old-local
hatch run odbc-both-local
hatch run core-local-no-docker
```

#### CI Testing (With Benchstore Upload)

```bash
hatch run core
hatch run python-universal
hatch run python-old
hatch run python-both
hatch run odbc-universal
hatch run odbc-old
hatch run odbc-both
```

### Command-Line Options

| Option | Description | Default |
|--------|-------------|---------|
| `--parameters-json` | Path to parameters JSON file | `parameters/parameters_perf_aws.json` |
| `--iterations` | Number of test iterations | `2` (or per-test marker) |
| `--warmup-iterations` | Number of warmup iterations | `1` (or per-test marker) |
| `--driver` | Driver to test | `core` |
| `--driver-type` | `universal`, `old`, or `both` | `universal` |
| `--upload-to-benchstore` | Upload metrics to Benchstore | `false` |
| `--local-benchstore-upload` | Use local auth for Benchstore | `false` |
| `--use-local-binary` | Use local binary (Core only) | `false` |

#### Examples with Custom Arguments

All hatch test scripts accept pytest arguments:

```bash
# Custom parameters file
hatch run core-local --parameters-json=parameters/my_parameters.json

# Custom iterations
hatch run python-universal-local --iterations=5 --warmup-iterations=2

# Specific test
hatch run core-local tests/test_fetch_1000000.py::test_fetch_string_1000000_rows

# Filter tests by name pattern
hatch run python-both-local -k "test_fetch_string"

# With Benchstore upload using local auth
hatch run core --upload-to-benchstore --local-benchstore-upload
```

### Utility Scripts

```bash
hatch run clean  # Remove cache directories and results
```

---

## Adding New Tests

1. Create test in `tests/` directory
2. Use `perf_test` fixture
3. Add appropriate markers for iterations
4. Extend driver images if needed

### Adding New Drivers

1. Create driver directory: `drivers/<driver_name>/`
2. Implement driver following the container input/output contract
3. Create `Dockerfile` and `build.sh`
4. Add hatch scripts to `pyproject.toml`
5. Update this README

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Test Runner (Python)                    │
│  - Test definitions                                             │
│  - Orchestrating test executions for selected drivers           │
│  - Collects and validates results                               │
│  - Uploads metrics to Benchstore                                │
└─────────────────────────┬───────────────────────────────────────┘
                          │
                          │ Creates & Runs
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│                  Driver Containers (Docker)                     │
│                                                                 │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌──────────┐   │
│  │   Core     │  │   Python   │  │    ODBC    │  │   JDBC   │   │
│  │  (Rust)    │  │            │  │            │  │          │   │
│  └────────────┘  └────────────┘  └────────────┘  └──────────┘   │
│                                                                 │
│  Each driver:                                                   │
│  - Receives configuration via environment variables             │
│  - Connects to Snowflake                                        │
│  - Executes setup queries                                       │
│  - Runs warmup iterations                                       │
│  - Executes test iterations                                     │
│  - Measures query and fetch times                               │
│  - Writes results to CSV files                                  │
│  - Writes run metadata                                          │
└─────────────────────────────────────────────────────────────────┘
```

## Driver Containers

Each driver image contains both the **universal driver** (built from this repository) and the **latest released old driver**. The `DRIVER_TYPE` environment variable controls which implementation is used:

- `DRIVER_TYPE=universal`: Uses the universal driver implementation
- `DRIVER_TYPE=old`: Uses the latest released production driver
- `DRIVER_TYPE=both`: Each test runs twice - first with universal driver, then with old driver
- Core driver only supports `universal` (no old implementation)

This allows performance comparison between the universal driver and the existing production driver within the same test run.

All drivers receive their configuration through **environment variables**. The runner sets these when creating containers.

### Required Environment Variables

| Variable | Type | Description | Example |
|----------|------|-------------|---------|
| `PARAMETERS_JSON` | JSON string | Snowflake connection parameters | See below |
| `SQL_COMMAND` | String | SQL query to execute | `"SELECT * FROM table LIMIT 1000000"` |
| `TEST_NAME` | String | Test and metric name | `"fetch_string_1000000_rows"` |
| `PERF_ITERATIONS` | Integer | Number of test iterations | `"3"` |
| `PERF_WARMUP_ITERATIONS` | Integer | Number of warmup iterations | `"1"` |

### Optional Environment Variables

| Variable | Type | Description | Default |
|----------|------|-------------|---------|
| `DRIVER_TYPE` | String | `"universal"` or `"old"` | `"universal"` |
| `SETUP_QUERIES` | JSON array | SQL queries to run before test (ARROW format is always first) | `["ALTER SESSION SET QUERY_RESULT_FORMAT = 'ARROW'"]` |

### PARAMETERS_JSON Format

The `PARAMETERS_JSON` environment variable must contain a JSON object with a `testconnection` key:

```json
{
  "testconnection": {
    "SNOWFLAKE_TEST_ACCOUNT": "myaccount",
    "SNOWFLAKE_TEST_HOST": "myaccount.snowflakecomputing.com",
    "SNOWFLAKE_TEST_USER": "testuser",
    "SNOWFLAKE_TEST_DATABASE": "testdb",
    "SNOWFLAKE_TEST_SCHEMA": "public",
    "SNOWFLAKE_TEST_WAREHOUSE": "compute_wh",
    "SNOWFLAKE_TEST_ROLE": "testrole",
    "SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS": [
      "-----BEGIN PRIVATE KEY-----",
      "...",
      "-----END PRIVATE KEY-----"
    ]
  }
}
```

### Expected Outputs

Each driver container must generate:

1. **CSV Results File**: `/results/<test_name>_<driver>_<type>_<timestamp>.csv`
   ```csv
   query_time_ms,fetch_time_ms
   1583.121061,21441.599846
   1812.227726,20262.201548
   ...
   ```
   Each row represents one test iteration. Values are in milliseconds with 6 decimal places.

2. **Metadata File**: `/results/run_metadata_<driver>_<type>.json`
   ```json
   {
     "driver": "python",
     "driver_type": "universal",
     "driver_version": "1.2.3",
     "server_version": "9.34.0",
     "run_timestamp": 1761734615
   }
   ```


## Test Structure

### Writing Tests

Tests are written using pytest with the `perf_test` fixture:

```python
@pytest.mark.iterations(3)
@pytest.mark.warmup_iterations(1)
def test_fetch_number_1000000_rows(perf_test):
    """Custom iterations via markers"""
    perf_test(
        sql_command="SELECT L_LINENUMBER::int FROM SNOWFLAKE_SAMPLE_DATA.TPCH_SF100.LINEITEM LIMIT 1000000"
    )

def test_with_additional_setup(perf_test):
    """Optional: Add additional setup queries"""
    perf_test(
        sql_command="SELECT * FROM my_table",
        setup_queries=[
            "ALTER SESSION SET QUERY_TAG = 'perf_test'"
        ]
    )
```

**Note**: ARROW format (`ALTER SESSION SET QUERY_RESULT_FORMAT = 'ARROW'`) is automatically enabled for all tests. Any provided `setup_queries` will be appended after the ARROW format query.

### Test Configuration Priority

Configuration values are resolved in the following priority order (highest to lowest):

1. **Command-line arguments**: `--iterations=5`
2. **Test-level markers**: `@pytest.mark.iterations(3)`
3. **Environment variables**: `PERF_ITERATIONS=2`
4. **Defaults**: `iterations=2`, `warmup_iterations=1`

## Results

### Results Directory Structure

```
results/
└── run_20251030_113045/
    ├── fetch_string_1000000_rows_python_universal_1761734615.csv
    ├── fetch_string_1000000_rows_python_old_1761734627.csv
    ├── fetch_number_1000000_rows_python_universal_1761734660.csv
    ├── fetch_number_1000000_rows_python_old_1761734671.csv
    ├── run_metadata_python_universal.json
    └── run_metadata_python_old.json
```

### CSV Format

Results CSV files contain per-iteration timing data:

```csv
query_time_s,fetch_time_s
1.583121,21.441600
1.812228,20.262202
1.799454,20.156388
```

**Columns**:
- `query_time_s`: Time to execute query and get initial response (seconds, 6 decimal places)
- `fetch_time_s`: Time to fetch all result data (seconds, 6 decimal places)

**Notes**:
- Each row represents one test iteration (warmup iterations are not included)
- Row number implicitly indicates iteration number (first data row = iteration 1)
- Total time can be calculated as `query_time_s + fetch_time_s`

### Metadata JSON Format

```json
{
  "driver": "python",
  "driver_type": "universal",
  "driver_version": "1.2.3",
  "server_version": "9.34.0",
  "run_timestamp": 1761734615
}
```

**Fields**:
- `driver`: Driver name ("python", "core", "odbc", etc.)
- `driver_type`: Implementation type ("universal" or "old")
- `driver_version`: Version string (may be "UNKNOWN" if not available)
- `server_version`: Snowflake server version
- `run_timestamp`: Unix timestamp (seconds since epoch)

### Benchstore Metrics

When uploading to Benchstore (with `--upload-to-benchstore`), each test uploads performance metrics that can be compared across drivers.

- **Consistent metric names**: All drivers use identical metric names for the same test (e.g., `fetch_string_1000000_rows.query_time_s`)
- **Tag-based separation**: Results are distinguished by tags (driver, version, cloud provider, etc.)
- **Cross-driver comparison**: This enables direct performance comparison between Core, Python, and ODBC drivers

**Example**: The test `test_fetch_string_1000000_rows` uploads:
- Metric name: `fetch_string_1000000_rows.query_time_s` (same for all drivers)
- Separated by tags:
  - Core: `DRIVER=core`, `DRIVER_VERSION=0.1.0`
  - Python Universal: `DRIVER=python`, `DRIVER_VERSION=0.1.0`
  - Python Old: `DRIVER=python_old`, `DRIVER_VERSION=3.12.0`

#### Benchstore Tags

The following tags are automatically attached to each metric:

| Tag | Description | Source | Example |
|-----|-------------|--------|---------|
| `BUILD_NUMBER` | CI build number or "LOCAL" | Jenkins `BUILD_NUMBER` env var | `"1234"` or `"LOCAL"` |
| `BRANCH_NAME` | Git branch name or "LOCAL" | Jenkins `BRANCH_NAME` env var | `"main"` or `"LOCAL"` |
| `DRIVER` | Driver name (with `_old` suffix for old driver) | Test configuration | `"python"`, `"core"`, `"odbc_old"` |
| `SERVER_VERSION` | Snowflake server version | Retrieved during connection | `"9.34.0"` |
| `DRIVER_VERSION` | Driver library version | See version detection below | `"0.1.0"` or `"UNKNOWN"` |
| `CLOUD_PROVIDER` | Cloud platform | Parameters filename | `"AWS"`, `"AZURE"`, `"GCP"` |

**Tag usage notes**:
- `CLOUD_PROVIDER` is extracted from the parameters filename (e.g., `parameters_perf_aws.json` → `"AWS"`)
- Old driver implementations have `_old` suffix in the `DRIVER` tag (e.g., `"python_old"`)
- Local test runs use `"LOCAL"` for build and branch information
- Tags enable filtering and grouping metrics in Benchstore for analysis and comparison

#### Driver Version Detection

How `DRIVER_VERSION` is determined for each driver:

| Driver | Universal Implementation | Old Implementation |
|--------|-------------------------|-------------------|
| **Core** | Uses compile-time `CARGO_PKG_VERSION` macro from `Cargo.toml` (`0.1.0`) | N/A (no old implementation) |
| **Python** | Uses `importlib.metadata.version("snowflake-connector-python-ud")` from installed package (`0.1.0`) | Uses `importlib.metadata.version("snowflake-connector-python")` from installed package |
| **ODBC** | `"UNKNOWN"` (SQLGetInfo not yet implemented) | Retrieved via `SQLGetInfo(SQL_DRIVER_VER)` from installed driver |

---

## Docker Builds Approach

The framework uses a multi-stage Docker build strategy with **cargo-chef** for Rust dependency caching (speeds up local builds after code changes).

### Shared Builder Image (`sf-core-builder`)

For ODBC and Python drivers, a shared base image is built first using `Dockerfile.sf_core_builder` to not repeat core building steps:

```bash
./drivers/build_sf_core_builder.sh
```

This creates an intermediate image containing Core libraries:
- `libsf_core.so` - Core Snowflake driver library
- `libsfodbc.so` - ODBC wrapper around `sf_core`

These libraries are copied into the final driver images:
- **Python**: Copies `libsf_core.so` → Used by `snowflake-connector-python-ud` package
- **ODBC**: Copies both `libsf_core.so` and `libsfodbc.so` → Loaded by unixODBC driver manager

