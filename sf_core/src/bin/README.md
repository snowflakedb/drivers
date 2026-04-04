# `collect_chunk_data`

CLI tool that captures raw Snowflake query chunk data to disk for offline
benchmarking and testing.

It connects to a Snowflake account, executes a query, and saves every chunk
(initial inline data + remote chunks) as binary files alongside a
`metadata.json` manifest that records row counts, sizes, and row-type schemas.

## Usage

```bash
cargo run --bin collect_chunk_data -- \
    --parameter-path parameters.json \
    --output-dir chunk_test_data \
    --format arrow \
    --sql "SELECT * FROM my_table LIMIT 1000000"
```

### Arguments

| Flag               | Default              | Description                                                                 |
|--------------------|----------------------|-----------------------------------------------------------------------------|
| `--sql`            | *(required)*         | SQL query to execute                                                        |
| `--parameter-path` | `$PARAMETER_PATH`    | Path to the test parameters JSON file (same format used by integration tests) |
| `--output-dir`     | `chunk_test_data`    | Directory where chunk files and metadata are written                        |
| `--format`         | `arrow`              | Result format (`arrow` or `json`); sets `PYTHON_CONNECTOR_QUERY_RESULT_FORMAT` session parameter |

### Parameters file

The tool reads Snowflake connection parameters from a JSON file with the same
structure used by integration tests:

```json
{
  "testconnection": {
    "SNOWFLAKE_TEST_ACCOUNT": "...",
    "SNOWFLAKE_TEST_USER": "...",
    "SNOWFLAKE_TEST_HOST": "...",
    "SNOWFLAKE_TEST_WAREHOUSE": "...",
    "SNOWFLAKE_TEST_DATABASE": "...",
    "SNOWFLAKE_TEST_SCHEMA": "...",
    "SNOWFLAKE_TEST_ROLE": "...",
    "SNOWFLAKE_TEST_PRIVATE_KEY_FILE": "..."
  }
}
```

## Helper script

`tests/test_data/test_data_generators/generate_chunks.sh` wraps this binary
to generate the standard benchmark datasets (1M rows, 15 columns from
`TPCH_SF100.LINEITEM`) in both Arrow and JSON formats.

## Output

The output directory will contain:

- `metadata.json` — manifest with format, chunk count, row types, and per-chunk sizes
- `initial.bin` — initial inline rowset (if present)
- `chunk_0.bin`, `chunk_1.bin`, … — raw (decompressed) remote chunk data
