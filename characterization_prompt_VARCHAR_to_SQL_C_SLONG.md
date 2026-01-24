# Characterization Test Generation Task

You are generating characterization tests for the Snowflake ODBC driver. Your goal is to create comprehensive tests that capture the EXACT behavior of the OLD (reference) Snowflake ODBC driver for a specific type conversion.

## Conversion to Test

- **Source Snowflake Type**: `VARCHAR`
- **Target SQL C Type**: `SQL_C_SLONG`

## Critical Context

This is a characterization test suite. The purpose is to:
1. **Capture the OLD driver's exact behavior** - including quirks, edge cases, and even bugs
2. **Document all behavior** - especially surprising or undocumented behavior

The penalty for behavioral differences is very high (data incidents), so be thorough.

## Your Task

Create a C++ test file at:
```
odbc_tests/tests/characterization/conversion/VARCHAR_to_SQL_C_SLONG.cpp
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

### 7. Review the tests at the end and add tests that fill the gaps detected
 - You want to see if there is anything missing
 - It's better to add a test or example that's not needed than regret it later
 - You don't want to cause a DATA INCIDENT!!!

## Code Structure Requirements

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
TEST_CASE("VARCHAR to SQL_C_SLONG - normal values", "[characterization][conversion][VARCHAR][SQL_C_SLONG]") {
    Connection conn;
    auto random_schema = Schema::use_random_schema(conn);
    
    auto stmt = conn.execute_fetch("SELECT <value1>::VARCHAR as c1, <value2>::VARCHAR as c2, ..., <value10>::VARCHAR as c10");
    
    // Verify conversion
    // Use SQLGetData directly assert on each collumn
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
TEST_CASE("VARCHAR to SQL_C_SLONG - overflow error", "[characterization][conversion][VARCHAR][SQL_C_SLONG]") {
    // Test setup ...
    
    // Assertions
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_SHORT, &value, sizeof(value), &indicator);
    
    // Check return code
    CHECK(ret == SQL_ERROR);  // or SQL_SUCCESS_WITH_INFO for truncation
    
    // Capture diagnostic info
    auto diag = get_diag_rec(stmt);
    REQUIRE(!diag.empty());
    INFO("SQLSTATE: " << diag[0].sqlState);
    INFO("Native Error: " << diag[0].nativeError);
    INFO("Message: " << diag[0].messageText);
    CHECK(diag[0].sqlState == "22003");  // Numeric value out of range
}
```

### Testing NULL Values
```cpp
TEST_CASE("VARCHAR to SQL_C_SLONG - NULL handling", "[characterization][VARCHAR][SQL_C_SLONG]") {
    // Test setup ...
    
    // Assertions
    char buffer[100];
    SQLLEN indicator;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_SLONG, buffer, sizeof(buffer), &indicator);
    
    CHECK(ret == SQL_SUCCESS);
    CHECK(indicator == SQL_NULL_DATA);
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
- `[VARCHAR]` - Source type tag (lowercase)
- `[SQL_C_SLONG]` - Target type tag (lowercase)

Example:
```cpp
TEST_CASE("VARCHAR to SQL_C_NUMERIC - precision 38 scale 10", "[characterization][varchar][sql_c_numeric]")
```

## Avoid antipatterns

Make sure we don't use any antipattern

### Always make assertions strict

Always assert and never test values with if statements.

Expected pattern:
```cpp
   REQUIRE(ret == SQL_SUCCESS);
```

Antipattern:
```cpp
   if (ret == SUCCESS) {
      // Some assertions
   }
   else {
      // Other assertions
   }
```

### Unfold loops

Expected pattern:
```cpp
   {
      // Assertions for 1
   }

   {
      // Assertions for 2
   }
```

Antipattern:
```cpp
   for (int i = 0; i < ...; i++) {
      // Assertion for i
   }
```

### Make assertions strict

Expected:
```cpp
   CHECK(vec.size() == <number>); // Check exact size
   CHECK(<condition-a>) // Make the assertion stricter
```

Antipattern:
```cpp
   CHECK(!v.empty());
   CHECK((<condition-a> || <condition-b>)); // We should assert either a or b
```

## Additional Notes

- Create ONE test file for this specific conversion
- Be thorough - test every edge case you can think of
- Document your findings in comments
- If the conversion is not supported, create a test that documents this with the expected error
- Merge test cases into one if possible, each connection establishment costs test run time
- Try to test at least 10 values in each test
- Don't create a table in each tests 
- Test type aliases for SQL_C_SLONG 

## Capture the exact behaviour of the old driver

Do this until tests pass:
1. Build and run the tests:
   ```bash
   RUN_CHARACTERIZATION=1 ./odbc_tests/run_reference.sh -R characterization_VARCHAR_to_SQL_C_SLONG
   ```
2. Make changes so that the test suite captures driver behaviour
   - Make assertion on all returns and output arguments
   - Make sure the assertions are strict as possible
      - Don't check for emptiness -> asssert size
      - Don't use (a || b) in assertion -> assert a or assert b
   - Make sure we don't use any antipatterns in tests
4. Repeat until the all tests pass

## Report on the task

Once the tests are ready create a VARCHAR_to_SQL_C_SLONG.md file that summarizes
- test coverage - summary of test suite
- key findings - interesting findings about old driver behaviour
Make sure that the md file is small and concise, the most important thing is to capture key findings

Now, please create the characterization test file for `VARCHAR` to `SQL_C_SLONG` conversion.
