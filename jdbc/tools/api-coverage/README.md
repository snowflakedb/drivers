# API Coverage Report (Old vs New JDBC Driver)

This tool compares API method coverage between:
- old driver: `snowflake-jdbc`
- new driver: `universal-driver/jdbc`

## Scope

The comparison baseline includes:
- JDBC interfaces from local JDK (`java.sql.*`, `javax.sql.*`) via `javap`
- Snowflake API interfaces from source (`net.snowflake.client.api.*`)

Implementation scan includes concrete classes under:
- `net.snowflake.client.internal`
- `net.snowflake.client.api`

## Categories

- `implemented`
  - method is present and not classified as unsupported/not implemented
- `unsupported_by_design`
  - old: method throws `SnowflakeLoggedFeatureNotSupportedException` or `SQLFeatureNotSupportedException`
  - new: method throws `SQLFeatureNotSupportedException`
- `not_implemented` (new only)
  - method throws `NotImplementedException`
  - or method is absent in new while present in old baseline
- `missing`
  - not used in leadership view
  - baseline methods that are missing are folded into `unsupported_by_design`

## Outputs

Default output files (from `universal-driver/jdbc`):
- `build/reports/jdbc_api_coverage_report.json`
- `build/reports/jdbc_api_method_comparison.csv`

CSV columns:
- `interface`
- `method_signature`
- `old_category`
- `new_category`
- `changed`

Leadership buckets in JSON:
- `done` = `implemented` + `unsupported_by_design`
- `remaining` = `not_implemented`
- `done_pct`, `remaining_pct` (short percentage view for leadership)

## Setup

```bash
python3 -m venv .venv-api-coverage
source .venv-api-coverage/bin/activate
python -m pip install javalang
```

## Run

From `universal-driver/jdbc`:

```bash
source .venv-api-coverage/bin/activate
python tools/api-coverage/api_coverage_report.py
```

With custom paths:

```bash
python tools/api-coverage/api_coverage_report.py \
  --old-root /path/to/snowflake-jdbc/src/main/java \
  --new-root /path/to/universal-driver/jdbc/src/main/java \
  --output-json /tmp/jdbc_api_coverage_report.json \
  --output-table-csv /tmp/jdbc_api_method_comparison.csv \
  --include-package-prefix net.snowflake.client.internal \
  --include-package-prefix net.snowflake.client.api
```

## Notes

- Totals are directly comparable because both drivers are projected to the same baseline.
- The parser normalizes Java array constructor method references (for example `Integer[]::new`)
  before `javalang` parsing. This does not change classification logic.
