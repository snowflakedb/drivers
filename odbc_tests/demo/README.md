# Universal Snowflake ODBC Driver - Advanced Demo

This demo showcases the capabilities of the Universal Snowflake ODBC Driver.

## Building

```bash
cd odbc_tests/cmake-build
cmake ..
make advanced_demo
```

## Running

```bash
PARAMETER_PATH=/path/to/parameters.json \
DRIVER_PATH=/path/to/libsfodbc.dylib \
./odbc_tests/cmake-build/demo/advanced_demo /path/to/parameters.json
```

## Features Demonstrated

### ✅ Working Features

1. **Connection Management**
   - Connects to Snowflake using ODBC driver
   - Displays connection time and session information

2. **Basic Queries**
   - Executes simple SELECT statements
   - Retrieves session metadata (user, role, database, etc.)

3. **Complex Aggregations**
   - Creates temporary tables
   - Inserts data
   - Performs GROUP BY aggregations
   - Displays formatted results

4. **Large Result Sets**
   - Generates 10,000+ rows using GENERATOR
   - Processes data in pages
   - Calculates statistics (min, max, average)
   - Reports throughput (rows/sec)

5. **File Operations (PUT)**
   - Creates temporary stages
   - Uploads files to Snowflake
   - Shows compression ratios
   - Displays upload progress

6. **Error Handling**
   - Comprehensive diagnostic information
   - SQLSTATE codes
   - Human-readable error messages

### ⚠️ Known Limitations

1. **Parameter Binding**
   - Prepared statements with parameters not yet fully supported
   - Workaround: Use direct SQL with embedded values

2. **GET Operations**
   - File downloads need cursor handling improvements
   - Workaround: Use PUT for uploads only

3. **Transaction Management**
   - Auto-commit control not yet implemented
   - Workaround: Use explicit BEGIN/COMMIT statements

4. **Complex Data Types**
   - VARIANT, ARRAY, OBJECT support is experimental
   - Some values may not display correctly

## Performance

The demo typically achieves:
- **Connection**: ~1-2 seconds
- **Simple queries**: ~1 second
- **Large result sets**: 3,000-4,000 rows/sec
- **File uploads**: Varies by size and compression

## Example Output

```
═══════════════════════════════════════════════════════════════
  Demo 4: Large Result Set - Pagination Demo
═══════════════════════════════════════════════════════════════

ℹ Generating 10000 rows using GENERATOR...

Processing pages:
  Page   1: Processed   1000 rows (407 rows/sec)
  Page   2: Processed   2000 rows (814 rows/sec)
  ...
  Page  10: Processed  10000 rows (4070 rows/sec)

Statistics:
  Total Rows:    10000
  Average Value: 50.23
  Min Value:     1
  Max Value:     100
  Total Time:    2457ms
  Throughput:    4070 rows/sec
✓ Processed 10000 rows successfully
```

## Troubleshooting

### Connection Fails
- Verify `PARAMETER_PATH` points to valid `parameters.json`
- Check `DRIVER_PATH` points to the correct `.dylib` file
- Ensure Snowflake credentials are correct

### Garbage Data in Output
- This is a known issue with string column handling
- Fix in progress for proper null-termination

### Slow Performance
- First query after connection may be slower (warehouse startup)
- Subsequent queries should be faster
- Large result sets benefit from Arrow format

## Architecture

The demo uses:
- **ODBC C API**: Standard database connectivity
- **Arrow Format**: Efficient columnar data transfer
- **Rust Core**: High-performance query execution
- **Multi-chunk Support**: Handles result sets of any size

