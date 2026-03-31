# Performance Testing Framework

## Table of Contents

- [Usage](#usage)
- [Adding New Tests](#adding-new-tests)
- [Tests with Recorded HTTP Traffic](#tests-with-recorded-http-traffic)
- [Architecture](#architecture)
- [Driver Containers](#driver-containers)
- [Results](#results)
- [Metrics Reference](#metrics-reference)
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
| `--preserve-mappings` | Keep WireMock mappings after tests (for debugging) | `false` (enabled in local runs) |
| `--reuse-mappings` | Reuse existing mappings directory (e.g., `run_20260115_120000`) | None (runs recording phase) |

### Local Performance Compare

When running locally via `hatch run *-local` scripts, a session summary is printed at the end of the run (after `✓ TESTS COMPLETED`). UD vs OLD comparisons are always shown when both drivers ran. `PERF_LOCAL_COMPARE=1` (set in those scripts) additionally enables comparisons against previous local runs and single-driver summaries.

**What is shown:**

- **UD vs OLD** (when `--driver-type=both`): percentage difference between Universal and Old driver from the current run. Always shown when both drivers ran, regardless of `PERF_LOCAL_COMPARE`.
- **vs last run** (requires `PERF_LOCAL_COMPARE=1` and at least 1 previous run): current median compared to the previous run's median.
- **vs all prev** (requires `PERF_LOCAL_COMPARE=1` and at least 2 previous runs): current median compared to the median across all previous runs.

History comparisons are silently skipped when there are no previous runs for `vs last run`, or fewer than 2 previous runs for `vs all prev`.

Metric used: `fetch_s` for SELECT tests, `query_s` for PUT/GET tests. Per-run values use the median over all iterations.

**Session summary format:**
```
================================================================================
SUMMARY
================================================================================

  select_string_1M_arrow_recorded_http  (python universal)
    UD vs OLD:    fetch_s  UD=0.327s  OLD=0.459s  UD is -28.7% (faster) than OLD
    vs last run:  fetch_s  0.459s -> 0.327s  -28.7%  faster  (run_20260326_122102)
    vs all prev:  fetch_s  median 0.367s -> 0.327s  -10.8%  faster  [N=17]

================================================================================
```

Previous runs are read from the local `results/` directory. Use `hatch run clean` to reset it.

---

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

from runner.test_types import PerfTestType

def test_put_files_12mx100(perf_test):
    """PUT/GET test: Upload files to Snowflake stage"""
    perf_test(
        test_type=PerfTestType.PUT_GET,
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
- PUT/GET tests use `test_type=PerfTestType.PUT_GET` and measure only the file operation time (no separate fetch phase)
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

## Tests with Recorded HTTP Traffic

Tests with recorded HTTP traffic use WireMock to record HTTP traffic to Snowflake and replay it for deterministic performance testing without requiring live Snowflake connections.

### How It Works

1. **Recording Phase**: Real Snowflake requests/responses are captured via WireMock proxy and saved as mappings
2. **Replay Phase**: Recorded mappings are replayed for consistent, repeatable performance measurements

### Running Tests with Recorded HTTP Traffic

```bash
# Run with fresh recording (both record and replay phases)
hatch run python-universal-local tests/test_select_1M_recorded_http.py::test_select_string_1M_arrow_recorded_http

# Reuse existing mappings (skip recording, only replay phase)
hatch run python-universal-local tests/test_select_1M_recorded_http.py::test_select_string_1M_arrow_recorded_http --reuse-mappings run_20260115_120000
```

### WireMock-Specific Parameters

| Parameter | Description | Use Case |
|-----------|-------------|----------|
| `--preserve-mappings` | Keep mappings after test completion | Debugging or reusing mappings later. Enabled by default in local runs. |
| `--reuse-mappings <dir>` | Skip recording phase and reuse existing mappings | Faster iteration when testing against the same recorded traffic (e.g., `--reuse-mappings run_20260115_120000`) |

**Note**: Mappings are stored in `mappings/<run_id>/<test_name>/` and can be found by checking the test output for the run ID.

### Key Details

- **Test naming**: Tests end with `_recorded_http` (e.g., `test_select_string_1M_arrow_recorded_http`)
- **Benchstore upload**: Only replay phase metrics are uploaded; recording phase results are for debugging only
- **Docker networking**: Test driver and WireMock containers run in a shared Docker network for communication
- **Server version**: Marked as "N/A" in replay mode for Old driver since tests don't connect to real Snowflake
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
│  - Starts memory timeline monitor (background thread, 100ms)    │
│  - Executes test iterations                                     │
│  - Measures query/fetch times, CPU time, and peak RSS           │
│  - Stops memory timeline monitor                                │
│  - Writes per-iteration results CSV and memory timeline CSV     │
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

1. **CSV Results File**: `/results/<driver_type>/<test_name>/<test_name>_<driver>_<type>_<timestamp>.csv`

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
    ├── run_metadata_python_universal.json
    ├── run_metadata_python_old.json
    ├── wiremock/
    │   ├── wiremock_stats_select_string_1M_arrow_recorded_http_record_universal_1761734600.csv
    │   └── wiremock_logs_select_string_1M_arrow_recorded_http_replay_universal_1761734610.log
    ├── universal/
    │   ├── _record/
    │   │   └── select_string_1M_arrow_recorded_http_record_python_universal_1761734605.csv
    │   ├── select_string_1M_arrow/
    │   │   ├── select_string_1M_arrow_python_universal_1761734615.csv
    │   │   └── memory_timeline_select_string_1M_arrow_python_universal_1761734615.csv
    │   └── select_number_1M_arrow/
    │       ├── select_number_1M_arrow_python_universal_1761734660.csv
    │       └── memory_timeline_select_number_1M_arrow_python_universal_1761734660.csv
    └── old/
        ├── select_string_1M_arrow/
        │   ├── select_string_1M_arrow_python_old_1761734627.csv
        │   └── memory_timeline_select_string_1M_arrow_python_old_1761734627.csv
        └── select_number_1M_arrow/
            ├── select_number_1M_arrow_python_old_1761734671.csv
            └── memory_timeline_select_number_1M_arrow_python_old_1761734671.csv
```

### CSV Format

Results CSV files contain per-iteration timing, CPU, and memory data with actual execution timestamps.

**For SELECT tests:**
```csv
timestamp_ms,query_s,fetch_s,row_count,cpu_time_s,peak_rss_mb
1762522370000,0.005432,1.583121,1000000,1.571032,236.2
1762522372000,0.005118,1.812228,1000000,1.798445,237.4
1762522374000,0.004987,1.799454,1000000,1.785123,236.1
```

**For PUT/GET tests:**
```csv
timestamp_ms,query_s,cpu_time_s,peak_rss_mb
1762522254000,6.595445,0.312456,85.3
1762522271000,4.385419,0.298234,85.1
1762522288000,5.123456,0.305678,85.2
```

**Columns**:
- `timestamp_ms`: Unix timestamp in milliseconds when the iteration completed
- `query_s`: Wall-clock time to execute the query (`cursor.execute()`) and get initial response (seconds)
- `fetch_s`: Wall-clock time to fetch all result rows via `fetchmany()` (seconds) — **SELECT tests only**
- `row_count`: Number of rows fetched — **SELECT tests only**
- `cpu_time_s`: CPU time consumed during the fetch phase (seconds, via `time.process_time()`) — see [Metrics Reference](#metrics-reference)
- `peak_rss_mb`: Process-wide peak Resident Set Size (MB, via `getrusage(RUSAGE_SELF).ru_maxrss`)

**Notes**:
- Each row represents one test iteration (warmup iterations are not included)
- PUT/GET tests have no separate fetch phase — `query_s` covers the entire file operation
- Timestamps are captured at the end of each iteration and uploaded to Benchstore for time-series analysis
- Total wall-clock time for SELECT tests = `query_s + fetch_s`

### Memory Timeline Files

In addition to per-iteration CSVs, a separate memory timeline CSV is generated per test. A background thread samples process memory at ~100ms intervals across all test iterations (excluding warmup).

**Filename pattern:** `memory_timeline_{test_name}_{driver}_{driver_type}_{timestamp}.csv`

```csv
timestamp_ms,rss_bytes,vm_bytes
1773060664000,175636480,536870912
1773060664100,196083712,536870912
1773060664200,247463936,536870912
```

**Columns**:
- `timestamp_ms`: Epoch time in milliseconds when the sample was taken
- `rss_bytes`: Resident Set Size — physical memory currently used by the process
- `vm_bytes`: Virtual Memory Size — total virtual address space of the process

---

## Metrics Reference

### Per-Iteration Metrics (Time Series)

These are recorded once per test iteration and uploaded to Benchstore as time-series sample points.

| Metric | Scope | Measures | Unit | Purpose |
|--------|-------|----------|------|---------|
| `query_s` | `cursor.execute()` | Wall-clock time for query submission and server response | seconds | Network latency + server processing time |
| `fetch_s` | `cursor.fetchmany()` loop | Wall-clock time to fetch and deserialize all rows | seconds | Data transfer + deserialization throughput |
| `cpu_time_s` | Fetch phase only (SELECT) / Execute phase (PUT/GET) | CPU time (user + system) consumed by the process | seconds | CPU efficiency of deserialization; excludes I/O waits and GIL-released time |
| `peak_rss_mb` | Entire process lifetime | Maximum physical memory (Resident Set Size) ever used | MB | Absolute memory ceiling; useful for memory budget tracking |

**`cpu_time_s` details:**
- Python: measured via `time.process_time()` (user + system CPU, all threads)
- ODBC/C++: measured via `getrusage(RUSAGE_SELF)` delta (user + system CPU, all threads)
- For SELECT tests: wraps only the fetch loop, excluding the query phase. This means `cpu_time_s` directly corresponds to `fetch_s` — the ratio `cpu_time_s / fetch_s` indicates CPU saturation during deserialization (1.0 = fully CPU-bound, <1.0 = some I/O waits)
- For PUT/GET tests: wraps the execute call since there is no separate fetch phase
- Both methods report process-wide CPU across all threads. For multi-threaded drivers, `cpu_time_s` may exceed `fetch_s` (the excess reflects work done by background threads)

**`peak_rss_mb` details:**
- Read from `getrusage(RUSAGE_SELF).ru_maxrss` (kernel-tracked `VmHWM`)
- This is the process-wide peak — it captures all memory from both the language wrapper and the Rust `sf_core` library (they share the same process)
- The value is monotonically non-decreasing across iterations (it's a lifetime high-water mark). To see per-iteration memory behavior, use the memory timeline

**Platform note:** `cpu_time_s` and `peak_rss_mb` work on both Linux and macOS. Memory timeline sampling (`/proc/self/statm`) is Linux-only — on macOS, the timeline is silently empty. All metrics are available on any Linux distribution (the APIs are kernel-level, not distro-specific).

### Memory Timeline Metrics (Time Series)

Sampled at ~100ms intervals by a background monitoring thread. Uploaded to Benchstore as time-series sample points. **Linux only** — relies on `/proc/self/statm` which is part of the Linux kernel's procfs (available on all distributions and architectures). On non-Linux platforms the monitor is silently disabled.

| Metric | Source | Unit | Purpose |
|--------|--------|------|---------|
| `rss_memory_mb` | `/proc/self/statm` (RSS pages × page size) | MB | Instantaneous physical memory at each sample point; shows memory shape across iterations |
| `vm_memory_mb` | `/proc/self/statm` (VM pages × page size) | MB | Virtual memory size; useful for detecting address space growth independent of physical memory |

**Timeline scope:** The monitor starts after warmup and stops after the last test iteration. Setup queries, connection setup, and result writing are excluded.

### Aggregate Metrics (Single Value per Run)

Computed from memory timeline data and uploaded to Benchstore as run aggregates.

| Metric | Computation | Unit | Detects |
|--------|-------------|------|---------|
| `rss_memory_delta_mb` | `max(rss) - min(rss)` across all timeline samples | MB | Working memory regression |
| `rss_memory_growth_mb` | `min(rss in last iteration) - min(rss in first iteration)` | MB | Memory leaks |

**`rss_memory_delta_mb`** measures the working memory swing during test execution — how much memory the driver allocates on top of its idle baseline to process results. Compare this value across builds to detect regressions: if a code change causes the driver to allocate more memory during query processing, this number increases.

**`rss_memory_growth_mb`** measures whether memory is being freed between iterations. It compares the idle (resting) RSS of the first iteration to the idle RSS of the last iteration. A value near 0 means memory is fully released. A positive value means memory is accumulating across iterations (potential leak). Requires at least 2 iterations (skipped for single-iteration runs).

The metric uses `min()` within each iteration's timeline segment to extract the idle baseline, rather than comparing the raw first and last RSS samples. This matters because the monitor can start or stop mid-iteration when RSS is at its peak (~152 MB instead of ~103 MB idle). A raw `end - start` comparison would be timing-dependent and could falsely report ~48 MB of "growth" when the driver actually freed all working memory. The `min()` approach reliably finds the idle baseline regardless of timing.

**How the three memory metrics work together:**

| Metric | What it guards | Example alert |
|--------|----------------|---------------|
| `peak_rss_mb` | Absolute memory ceiling | "Process exceeded 256 MB budget" |
| `rss_memory_delta_mb` | Per-test working memory | "Decoding now uses 80 MB instead of 48 MB" |
| `rss_memory_growth_mb` | Memory leak across iterations | "Idle RSS grew by 10 MB over 5 iterations" |

### Benchstore Metrics

When uploading to Benchstore (with `--upload-to-benchstore`), each test uploads performance metrics that can be compared across drivers.

- **Consistent metric names**: All drivers use identical metric names for the same test (e.g., `select_string_1000000_rows_query_s`)
- **Tag-based separation**: Results are distinguished by tags (driver, version, cloud provider, architecture, etc.)

**Example**: The test `test_select_string_1000000_rows` uploads these time-series metrics per iteration:
- `select_string_1000000_rows_query_s` — query execution time
- `select_string_1000000_rows_fetch_s` — data fetching time
- `select_string_1000000_rows_cpu_time_s` — CPU time during fetch
- `select_string_1000000_rows_peak_rss_mb` — peak process memory

And these memory timeline metrics (sampled at ~100ms):
- `select_string_1000000_rows_rss_memory_mb` — RSS at each sample point
- `select_string_1000000_rows_vm_memory_mb` — virtual memory at each sample point

And these run aggregates:
- `select_string_1000000_rows_rss_memory_delta_mb` — max-min RSS delta (working memory swing)
- `select_string_1000000_rows_rss_memory_growth_mb` — idle RSS growth between first and last iteration (leak indicator)

**PUT/GET Tests**: Tests like `test_put_files_12mx100` upload:
- `put_files_12mx100_query_s` — file operation time
- `put_files_12mx100_cpu_time_s` — CPU time during operation
- `put_files_12mx100_peak_rss_mb` — peak process memory
- Plus memory timeline, delta, and growth metrics (same as above)

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
| `JENKINS_NODE` | Jenkins node label | Jenkins `JENKINS_NODE_LABEL` env var | `"regular-memory-node-snowos"` |
| `DOCKER_MEMORY` | Container memory limit | Docker resource configuration | `"4g"` |
| `DOCKER_CPU` | Container CPU limit | Docker resource configuration | `"2.0"` |
| `NODE_CPU_MODEL` | Host CPU model name | `/proc/cpuinfo` | `"Intel_R__Xeon_R__Platinum_8175M"` |
| `NODE_CPU_CORES` | Physical CPU cores | `/proc/cpuinfo` | `"16"` |
| `NODE_CPU_THREADS` | Logical CPU threads | `os.cpu_count()` | `"32"` |
| `NODE_MEMORY_GB` | Total host RAM (GB) | `/proc/meminfo` | `"64"` |
| `NODE_CPU_MAX_MHZ` | CPU max frequency (MHz) | `cpufreq` / `lscpu` | `"3500"` |
| `NODE_L3_CACHE` | L3 cache size | `sysfs` / `lscpu` | `"54M"` |
| `NODE_INSTANCE_TYPE` | EC2 instance type (AWS only) | EC2 Instance Metadata Service (IMDSv2/v1) | `"m5.4xlarge"` |

**Notes**:
- `CLOUD_PROVIDER` extracted from parameters filename (e.g., `parameters_perf_aws.json` → `"AWS"`)
- Old drivers have `_old` suffix (e.g., `"python_old"`)
- Local runs use `"LOCAL"` for build and branch tags
- **Node hardware tags** (`NODE_*`) help identify performance variance caused by Jenkins node heterogeneity — different physical machines in the pool may have different CPU models, memory sizes, and cache configurations

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

