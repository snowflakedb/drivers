# Performance Testing Framework

## Table of Contents

- [Usage](#usage)
- [Adding New Tests](#adding-new-tests)
- [Architecture](#architecture)
- [Driver Containers](#driver-containers)
- [Results](#results)
- [Docker Builds Approach](#docker-builds-approach)

---

## Usage

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

### Cloud Provider Selection

Default: AWS. Available: `aws`, `azure`, `gcp`

```bash
# Use --cloud flag
hatch run core-local --cloud=azure

# Use environment variable
CLOUD=azure hatch run python-both-local

# Use explicit path
hatch run core-local --parameters-json=parameters/parameters_perf_azure.json
```

### Command-Line Options

| Option | Description | Default |
|--------|-------------|---------|
| `--cloud` | Cloud provider: `aws`, `azure`, or `gcp` | `aws` |
| `--parameters-json` | Path to parameters JSON file | Auto-selected based on `--cloud` |
| `--iterations` | Number of test iterations | `5` (or per-test marker) |
| `--warmup-iterations` | Number of warmup iterations | `0` (or per-test marker) |
| `--driver` | Driver to test | `core` |
| `--driver-type` | `universal`, `old`, or `both` | `universal` |
| `--upload-to-benchstore` | Upload metrics to Benchstore | `false` |
| `--local-benchstore-upload` | Use local auth for Benchstore | `false` |
| `--use-local-binary` | Use local binary (Core only) | `false` |

#### Examples with Custom Arguments

```bash
# Different cloud
hatch run core-local --cloud=azure

# Custom iterations
hatch run python-universal-local --iterations=10 --warmup-iterations=2

# Specific test
hatch run core-local tests/test_select_1M.py::test_select_string_1M_arrow_fetch_s

# Filter by pattern
hatch run python-both-local -k "1M"

# Combined options
hatch run python-universal-local --cloud=azure --iterations=10 -k "1M"
```

### Utility Scripts

```bash
hatch run clean  # Remove cache directories and results
```

---

## Adding New Tests

**Quick steps:**
1. Create test in `tests/` directory
2. Use `perf_test` fixture
3. Add appropriate markers for iterations
4. Extend driver images if needed

### Writing Tests

Tests are written using pytest with the `perf_test` fixture:

```python
@pytest.mark.iterations(3)
@pytest.mark.warmup_iterations(1)
def test_select_number_1000000_rows(perf_test):
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

from runner.test_types import TestType

def test_put_files_12mx100(perf_test):
    """PUT/GET test: Upload files to Snowflake stage"""
    perf_test(
        test_type=TestType.PUT_GET,
        s3_download_url="s3://sfc-eng-data/ecosystem/12Mx100/",
        setup_queries=[
            "CREATE TEMPORARY STAGE put_test_stage"
        ],
        sql_command=(
            "PUT file:///put_get_files/* @put_test_stage "
            "AUTO_COMPRESS=FALSE SOURCE_COMPRESSION=NONE overwrite=true"
        )
    )
```

**Notes**: 
- **SELECT tests**: ARROW format (`ALTER SESSION SET QUERY_RESULT_FORMAT = 'ARROW'`) is added to any provided `setup_queries`.
- **PUT/GET tests**: `USE DATABASE {database}` is added to any provided `setup_queries`. This is required for `CREATE TEMPORARY STAGE` operations which need a database context.
- PUT/GET tests use `test_type=TestType.PUT_GET` and measure only the file operation time (no separate fetch phase)
- The `s3_download_url` parameter triggers automatic download of test files from S3 before test execution

### Test Configuration Priority

**Cloud/Parameters** (highest to lowest):
1. `--parameters-json=path/to/file.json`
2. `--cloud=azure`
3. `CLOUD` environment variable
4. Default: `aws`

**Iterations/Warmup** (highest to lowest):
1. Command-line: `--iterations=10`
2. Test marker: `@pytest.mark.iterations(5)`
3. Environment: `PERF_ITERATIONS=3`
4. Defaults: `iterations=5`, `warmup_iterations=0`

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
| `TEST_NAME` | String | Test and metric name | `"select_string_1000000_rows"` |
| `PERF_ITERATIONS` | Integer | Number of test iterations | `"3"` |
| `PERF_WARMUP_ITERATIONS` | Integer | Number of warmup iterations | `"1"` |

### Optional Environment Variables

| Variable | Type | Description | Default |
|----------|------|-------------|---------|
| `DRIVER_TYPE` | String | `"universal"` or `"old"` | `"universal"` |
| `TEST_TYPE` | String | `"select"` or `"put_get"` | `"select"` |
| `SETUP_QUERIES` | JSON array | SQL queries to run before test. For SELECT tests, ARROW format is prepended. For PUT/GET tests, `USE DATABASE` is prepended. | `[]` |

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
   
   See [CSV Format](#csv-format) section for detailed format specification.

2. **Metadata File**: `/results/run_metadata_<driver>_<type>.json`
   ```json
   {
     "driver": "python",
     "driver_type": "universal",
     "driver_version": "1.2.3",
     "server_version": "9.34.0",
     "architecture": "arm64",
     "os": "Debian_13",
     "run_timestamp": 1761734615
   }
   ```


## Results

### Results Directory Structure

```
results/
└── run_20251030_113045/
    ├── select_string_1000000_rows_python_universal_1761734615.csv
    ├── select_string_1000000_rows_python_old_1761734627.csv
    ├── select_number_1000000_rows_python_universal_1761734660.csv
    ├── select_number_1000000_rows_python_old_1761734671.csv
    ├── run_metadata_python_universal.json
    └── run_metadata_python_old.json
```

### CSV Format

Results CSV files contain per-iteration timing data with actual execution timestamps.

**For SELECT tests:**
```csv
timestamp,query_s,fetch_s
1762522370,1.583121,21.441600
1762522392,1.812228,20.262202
1762522414,1.799454,20.156388
```

**For PUT/GET tests:**
```csv
timestamp,query_s
1762522254,6.595445
1762522271,4.385419
1762522288,5.123456
```

**Columns**:
- `timestamp`: Unix timestamp (seconds since epoch) when the iteration was executed
- `query_s`: Time to execute query and get initial response (seconds, 6 decimal places)
- `fetch_s`: Time to fetch all result data (seconds, 6 decimal places) - **only for SELECT tests**

**Notes**:
- Each row represents one test iteration (warmup iterations are not included)
- PUT/GET tests only measure `query_s` since file operations don't have a separate fetch phase
- Timestamps are captured at the end of each iteration and uploaded to Benchstore for accurate time-series analysis
- Total time for SELECT tests can be calculated as `query_s + fetch_s`

### Benchstore Metrics

When uploading to Benchstore (with `--upload-to-benchstore`), each test uploads performance metrics that can be compared across drivers.

- **Consistent metric names**: All drivers use identical metric names for the same test (e.g., `select_string_1000000_rows_query_s`)
- **Tag-based separation**: Results are distinguished by tags (driver, version, cloud provider, architecture, etc.)

**Example**: The test `test_select_string_1000000_rows` uploads:
- Metric names: 
  - `select_string_1000000_rows_query_s` (query execution time)
  - `select_string_1000000_rows_fetch_s` (data fetching time)

**PUT/GET Tests**: Tests like `test_put_files_12mx100` only upload:
- Metric name: `put_files_12mx100_query_s` (query execution time)

#### Benchstore Tags

The following tags are automatically attached to each metric:

| Tag | Description | Source | Example |
|-----|-------------|--------|---------|
| `BUILD_NUMBER` | CI build number or "LOCAL" | Jenkins `BUILD_NUMBER` env var | `"1234"` or `"LOCAL"` |
| `BRANCH_NAME` | Git branch name or "LOCAL" | Jenkins `BRANCH_NAME` env var | `"main"` or `"LOCAL"` |
| `DRIVER` | Driver name (with `_old` suffix for old driver) | Test configuration | `"python"`, `"core"`, `"odbc_old"` |
| `DRIVER_VERSION` | Driver library version | See version detection below | `"0.1.0"` or `"UNKNOWN"` |
| `BUILD_RUST_VERSION` | Rust compiler version that built the code | Build-time detection | `"1.75"`, `"NA"` for non-Rust |
| `RUNTIME_LANGUAGE_VERSION` | Runtime language version for interpreted languages | Runtime detection | `"3.13"` for Python, `"NA"` for compiled code |
| `SERVER_VERSION` | Snowflake server version | Retrieved during connection | `"9.34.0"` |
| `CLOUD_PROVIDER` | Cloud platform | Parameters filename | `"AWS"`, `"AZURE"`, `"GCP"` |
| `REGION` | Cloud region | Extracted from host | `"us-west-2"`, `"east-us-2"` |
| `ARCHITECTURE` | CPU architecture | Detected from system | `"x86_64"`, `"arm64"` |
| `OS` | Operating system | Detected from system | `"Debian_13"`, `"Darwin_24.6.0"` |
| `TYPE` | Test execution type | For now e2e, will be expanded when tests with recorded http traffic will be added | `"e2e"` (end-to-end) |
| `JENKINS_NODE` | Jenkins node label | Jenkins `JENKINS_NODE_LABEL` env var | `"regular-memory-node-snowos"` |
| `DOCKER_MEMORY` | Container memory limit | Docker resource configuration | `"4g"` |
| `DOCKER_CPU` | Container CPU limit | Docker resource configuration | `"2.0"` |

**Notes**:
- `CLOUD_PROVIDER` extracted from parameters filename (e.g., `parameters_perf_aws.json` → `"AWS"`)
- Old drivers have `_old` suffix (e.g., `"python_old"`)
- Local runs use `"LOCAL"` for build and branch tags

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

