# ODBC Driver

## Configuration via `sf.odbc.ini`

On startup the driver reads a single `sf.odbc.ini` file. The first existing
location wins:

1. `$SF_ODBC_INI` (explicit override; useful in tests and CI),
2. `<config_dir>/snowflake/sf.odbc.ini` (e.g. `~/Library/Application Support/snowflake/sf.odbc.ini` on macOS, `~/.config/snowflake/sf.odbc.ini` on Linux),
3. `~/.snowflake/sf.odbc.ini`.

On Unix the file must be `chmod 600` (owner read/write only); otherwise the
driver logs a warning and falls back to defaults. The file is read once,
during environment allocation, and the snapshot is shared across all
subsystems.

All keys live in the unnamed top-level section and are matched
case-insensitively. The logging subsystem recognises:

| Key                  | Type    | Description                                                  |
|----------------------|---------|--------------------------------------------------------------|
| `LogLevel`           | enum    | `OFF`, `ERROR`, `WARN`, `INFO` (default), `DEBUG`, `TRACE`.  |
| `LogPath`            | path    | Directory for the rolling log file.                          |
| `LogFile`            | string  | Log file name (default `snowflake_odbc.log`).                |
| `LogMaxSize`         | bytes   | Per-file size cap before rotation.                           |
| `LogMaxCount`        | integer | Number of rotated files to retain.                           |
| `LogRotation`        | enum    | `none`, `hourly`, `daily`.                                   |
| `LogEnabled`         | bool    | Master switch; defaults to `true`.                           |
| `LogQueryText`       | bool    | Log the SQL text of executed statements.                     |
| `LogQueryParameters` | bool    | Log bound parameter values.                                  |
| `ErrorTraceEnabled`  | bool    | Capture error traces alongside the diagnostic record.        |

Unknown keys are silently ignored by the logging subsystem and made
available to other subsystems through the shared INI snapshot.

## Testing

ODBC tests are written in C++ using CMake and Catch2 framework.

### Prerequisites

Before running tests, ensure you have:
- Set up credentials (see main [README.md](../README.md) for setup instructions)
- CMake 3.10 or later
- C++17 compatible compiler
- coreutils (for `nproc`), unixodbc (for `odbc_config`) from `brew`
- (Optional) ccache for faster rebuilds: `brew install ccache`

When ccache is installed, the build scripts automatically use it as the compiler launcher.

### Local Testing (macOS/Linux)

```bash
# Build and run tests against new ODBC driver
./odbc_tests/run.sh

# Run specific tests
./odbc_tests/run.sh -R "suite_name"
```

### Reference Testing (Docker)

```bash
# Run tests against official Snowflake ODBC driver
./odbc_tests/run_reference.sh

# Pass specific test arguments
./odbc_tests/run_reference.sh -R "suite_name"
```
