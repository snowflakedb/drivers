# Characterization Test Generation Task

You are generating characterization tests for the Snowflake ODBC driver. Your goal is to create comprehensive tests that capture the EXACT behavior of the OLD (reference) Snowflake ODBC driver for a specific type conversion.

## Conversion to Test

- **Source Snowflake Type**: `{{SNOWFLAKE_TYPE}}`
- **Target SQL C Type**: `{{SQL_C_TYPE}}`

## Critical Context

This is a characterization test suite. The purpose is to:
1. **Capture the OLD driver's exact behavior** - including quirks, edge cases, and even bugs
2. **Ensure the NEW driver behaves identically** - any deviation could cause data incidents
3. **Document all behavior** - especially surprising or undocumented behavior

The penalty for behavioral differences is very high (data incidents), so be thorough.

## Your Task

Create a C++ test file at:
```
odbc_tests/tests/characterization/conversion/{{SNOWFLAKE_TYPE}}_to_{{SQL_C_TYPE}}.cpp
```

Use lowercase and underscores for the filename (e.g., `varchar_to_sql_c_numeric.cpp`).

## Test Categories to Cover

### 1. Valid Conversions - Normal Values
- Typical values that should convert successfully
- Various lengths/sizes within normal ranges

### 2. Boundary Values
- Minimum and maximum values for the target type
- Values at precision/scale limits
- Empty values (empty string, zero, etc.)

### 3. Edge Cases
- NULL values (check SQL_NULL_DATA indicator)
- Special numeric values: NaN, Infinity, -Infinity (if applicable)
- Very long strings (if applicable)
- Unicode characters (if applicable)
- Leading/trailing whitespace
- Scientific notation (for numeric conversions)

### 4. Precision and Scale Variations (for numeric types)
- Different precision values (1, 10, 18, 38)
- Different scale values (0, 2, 10)
- Loss of precision scenarios

### 5. Buffer Size Scenarios
- Exact buffer size match
- Buffer larger than needed
- Buffer smaller than needed (truncation)
- Zero-length buffer
- Check `str_len_or_ind` pointer values in all cases

### 6. Error Conditions
- Invalid conversions that should fail
- Overflow conditions
- Capture exact SQLSTATE codes (e.g., "22003" for numeric overflow)
- Capture exact error messages

## Code Structure Requirements

Follow the existing patterns in `odbc_tests/tests/datatype_tests/number_tests.cpp`:

```cpp
#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "Schema.hpp"
#include "get_data.hpp"
#include "get_diag_rec.hpp"
#include "macros.hpp"
#include "test_setup.hpp"

// Use descriptive test names that include the conversion being tested
TEST_CASE("{{SNOWFLAKE_TYPE}} to {{SQL_C_TYPE}} - normal values", "[characterization][{{SNOWFLAKE_TYPE}}][{{SQL_C_TYPE}}]") {
    Connection conn;
    auto random_schema = Schema::use_random_schema(conn);
    
    // Create table with specific column type
    conn.execute("DROP TABLE IF EXISTS test_char_{{SNOWFLAKE_TYPE}}_{{SQL_C_TYPE}}");
    conn.execute("CREATE TABLE test_char_{{SNOWFLAKE_TYPE}}_{{SQL_C_TYPE}} (col {{SNOWFLAKE_TYPE}})");
    
    // Insert test values
    conn.execute("INSERT INTO test_char_{{SNOWFLAKE_TYPE}}_{{SQL_C_TYPE}} VALUES (...)");
    
    auto stmt = conn.execute_fetch("SELECT col FROM test_char_{{SNOWFLAKE_TYPE}}_{{SQL_C_TYPE}}");
    
    // Verify conversion
    // Use SQLGetData directly to have full control over buffer sizes
}
```

## Helper Functions to Use

1. **Connection class** (`Connection.hpp`):
   - `conn.execute(query)` - Execute SQL
   - `conn.execute_fetch(query)` - Execute and fetch first row
   - `conn.createStatement()` - Create statement handle

2. **get_data template** (`get_data.hpp`):
   - `get_data<SQL_C_TYPE>(stmt, col)` - Simple get with default buffer

3. **Diagnostic records** (`get_diag_rec.hpp`):
   - `get_diag_rec(handle)` - Get all diagnostic records after error

4. **Direct SQLGetData** for buffer size testing:
```cpp
char buffer[100];
SQLLEN indicator;
SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
```

## Important Testing Patterns

### Testing for Expected Errors
```cpp
TEST_CASE("{{SNOWFLAKE_TYPE}} to {{SQL_C_TYPE}} - overflow error", "[characterization][{{SNOWFLAKE_TYPE}}][{{SQL_C_TYPE}}]") {
    Connection conn;
    auto random_schema = Schema::use_random_schema(conn);
    
    // Setup with value that will overflow
    conn.execute("DROP TABLE IF EXISTS test_overflow");
    conn.execute("CREATE TABLE test_overflow (col NUMBER(38,0))");
    conn.execute("INSERT INTO test_overflow VALUES (99999999999999999999)");
    
    auto stmt = conn.execute_fetch("SELECT col FROM test_overflow");
    
    // Attempt conversion that should fail
    SQLSMALLINT value;
    SQLLEN indicator;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_SHORT, &value, sizeof(value), &indicator);
    
    // Check return code
    CHECK(ret == SQL_ERROR);  // or SQL_SUCCESS_WITH_INFO for truncation
    
    // Capture diagnostic info
    auto diag = get_diag_rec(stmt);
    REQUIRE(!diag.empty());
    CHECK(diag[0].sqlState == "22003");  // Numeric value out of range
    
    // Document the actual values observed
    INFO("SQLSTATE: " << diag[0].sqlState);
    INFO("Native Error: " << diag[0].nativeError);
    INFO("Message: " << diag[0].messageText);
}
```

### Testing NULL Values
```cpp
TEST_CASE("{{SNOWFLAKE_TYPE}} to {{SQL_C_TYPE}} - NULL handling", "[characterization][{{SNOWFLAKE_TYPE}}][{{SQL_C_TYPE}}]") {
    Connection conn;
    auto random_schema = Schema::use_random_schema(conn);
    
    conn.execute("DROP TABLE IF EXISTS test_null");
    conn.execute("CREATE TABLE test_null (col {{SNOWFLAKE_TYPE}})");
    conn.execute("INSERT INTO test_null VALUES (NULL)");
    
    auto stmt = conn.execute_fetch("SELECT col FROM test_null");
    
    char buffer[100];
    SQLLEN indicator;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, {{SQL_C_TYPE}}, buffer, sizeof(buffer), &indicator);
    
    CHECK(ret == SQL_SUCCESS);
    CHECK(indicator == SQL_NULL_DATA);
}
```

### Testing Buffer Truncation
```cpp
TEST_CASE("{{SNOWFLAKE_TYPE}} to {{SQL_C_TYPE}} - buffer truncation", "[characterization][{{SNOWFLAKE_TYPE}}][{{SQL_C_TYPE}}]") {
    Connection conn;
    auto random_schema = Schema::use_random_schema(conn);
    
    conn.execute("DROP TABLE IF EXISTS test_truncation");
    conn.execute("CREATE TABLE test_truncation (col VARCHAR(100))");
    conn.execute("INSERT INTO test_truncation VALUES ('This is a long string that will be truncated')");
    
    auto stmt = conn.execute_fetch("SELECT col FROM test_truncation");
    
    // Use small buffer
    char buffer[10];
    SQLLEN indicator;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
    
    // Document exact behavior
    INFO("Return code: " << ret);
    INFO("Indicator: " << indicator);
    INFO("Buffer contents: " << std::string(buffer, std::min((SQLLEN)sizeof(buffer)-1, indicator)));
    
    // Verify behavior (adjust based on what OLD driver actually does)
    CHECK(ret == SQL_SUCCESS_WITH_INFO);
    // indicator should contain the full length of the data
}
```

## Workflow Requirements

1. **First, run tests against OLD driver**:
   - Build with `DRIVER_TYPE=OLD`
   - Characterization tests are skipped by default - set `RUN_CHARACTERIZATION=1` to run them:
     ```bash
     cd odbc_tests
     cmake -B cmake-build -DDRIVER_TYPE=OLD .
     cmake --build cmake-build
     RUN_CHARACTERIZATION=1 ctest --test-dir cmake-build -R characterization --output-on-failure
     ```
   - Tests MUST pass against the old driver before proceeding

2. **Document observed behavior**:
   - Add comments explaining any surprising behavior
   - Note exact SQLSTATE codes and error messages

3. **If you discover behavior differences** (when later testing NEW driver):
   - Use `OLD_DRIVER_ONLY("BD#N")` and `NEW_DRIVER_ONLY("BD#N")` macros
   - Document in `odbc_tests/BehaviorDifferences.yaml`

## Test Naming Convention

Use descriptive test names with tags:
- `[characterization]` - Always include this tag
- `[{{SNOWFLAKE_TYPE}}]` - Source type tag (lowercase)
- `[{{SQL_C_TYPE}}]` - Target type tag (lowercase)

Example:
```cpp
TEST_CASE("VARCHAR to SQL_C_NUMERIC - precision 38 scale 10", "[characterization][varchar][sql_c_numeric]")
```

## Additional Notes

- Create ONE test file for this specific conversion
- Be thorough - test every edge case you can think of
- Document your findings in comments
- If the conversion is not supported, create a test that documents this with the expected error

## After Creating Tests

1. The CMakeLists.txt at `odbc_tests/tests/characterization/conversion/CMakeLists.txt` automatically picks up all `.cpp` files - no manual edits needed
2. Each `.cpp` file becomes a separate test executable named `characterization_<filename>`
3. Build and run the tests:
   ```bash
   cd odbc_tests
   cmake -B cmake-build -DDRIVER_TYPE=OLD .
   cmake --build cmake-build
   RUN_CHARACTERIZATION=1 ctest --test-dir cmake-build -R characterization --output-on-failure
   ```
4. Commit the changes with a descriptive message

Now, please create the characterization test file for `{{SNOWFLAKE_TYPE}}` to `{{SQL_C_TYPE}}` conversion.
