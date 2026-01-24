# Characterization Test Generation Task

You are generating characterization tests for the Snowflake ODBC driver. Your goal is to create comprehensive tests that capture the EXACT behavior of the OLD (reference) Snowflake ODBC driver for a specific type conversion.

## Conversion to Test

- **Source Snowflake Type**: `{{SNOWFLAKE_TYPE}}`
- **Target SQL C Type**: `{{SQL_C_TYPE}}`

## Critical Context

This is a characterization test suite. The purpose is to:
1. **Capture the OLD driver's exact behavior** - including quirks, edge cases, and even bugs
2. **Document all behavior** - especially surprising or undocumented behavior

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

### 7. Impossible values
- Find value class that seem impossible to convert
- For VARIANT to SQL_TYPE_DATE: Can you convert VARIANT that contains date fields to SQL_TYPE_DATE?
- For VARCHAR to C_BIT: Can you convert VARCHAR containing date to C_BIT?
- Think of values that we might test to generate more edge / impossible cases

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
TEST_CASE("{{SNOWFLAKE_TYPE}} to {{SQL_C_TYPE}} - normal values", "[characterization][conversion][{{SNOWFLAKE_TYPE}}][{{SQL_C_TYPE}}]") {
    Connection conn;
    auto random_schema = Schema::use_random_schema(conn);
    
    auto stmt = conn.execute_fetch("SELECT <value1>::{{SNOWFLAKE_TYPE}} as c1, <value2>::{{SNOWFLAKE_TYPE}} as c2, ..., <value5>::{{SNOWFLAKE_TYPE}} as c5");
    
    // Verify conversion
    // Use SQLGetData directly assert on each collumn
}
```

## Helper Functions to Use

1. **Connection class** (`Connection.hpp`):
   - `conn.execute(query)` - Execute SQL
   - `conn.execute_fetch(query)` - Execute and fetch first row
   - `conn.createStatement()` - Create statement handle

2. **Custom assertions** (`macros.hpp`):
   - `CHECK_ODBC(ret, handle)`
3. **Direct SQLGetData** for buffer size testing:
```cpp
char buffer[100];
SQLLEN indicator;
SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
```

## Important Testing Patterns

### Testing for Expected Errors
```cpp
TEST_CASE("{{SNOWFLAKE_TYPE}} to {{SQL_C_TYPE}} - overflow error", "[characterization][conversion][{{SNOWFLAKE_TYPE}}][{{SQL_C_TYPE}}]") {
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
TEST_CASE("{{SNOWFLAKE_TYPE}} to {{SQL_C_TYPE}} - NULL handling", "[characterization][{{SNOWFLAKE_TYPE}}][{{SQL_C_TYPE}}]") {
    // Test setup ...
    
    // Assertions
    char buffer[100];
    SQLLEN indicator;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, {{SQL_C_TYPE}}, buffer, sizeof(buffer), &indicator);
    
    CHECK(ret == SQL_SUCCESS);
    CHECK(indicator == SQL_NULL_DATA);
}
```

## Workflow Requirements

## Test Naming Convention

Use descriptive test names with tags:
- `[characterization]` - Always include this tag
- `[{{SNOWFLAKE_TYPE}}]` - Source type tag (lowercase)
- `[{{SQL_C_TYPE}}]` - Target type tag (lowercase)
- `[conversion]` - Always include conversion tag

Example:
```cpp
TEST_CASE("VARCHAR to SQL_C_NUMERIC - precision 38 scale 10", "[characterization][conversion][varchar][sql_c_numeric]")
```

## Avoid antipatterns

Make sure we don't use any antipattern

### Always make assertions strict

Always assert and never test values with if statements.

Expected pattern:
```cpp
   CHECK_ODBC_CODE(ret, stmt, SQL_SUCCESS) // equivalent to REQUIRE(ret == SQL_SUCCESS), but with extra debug
```

Antipattern:
```cpp
   if (ret == SQL_SUCCESS) { // or == SQL_ERROR
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

## Guidelines

- **Document observed behavior**:
   - Add comments explaining any surprising behavior
   - Note exact SQLSTATE codes and error messages
- Create ONE test file for this specific conversion
- Be thorough - test every edge case you can think of
- Document your findings in comments
- If the conversion is not supported, create a test that documents this with the expected error
- Merge test cases into one if possible, each connection establishment costs test run time
- Try to test at least 5 values in each test
- Don't create a table in each tests 
- Test type aliases for {{SQL_C_TYPE}} 

## Capture the exact behaviour of the old driver

Do this until tests pass:
1. Build and run the tests:
   ```bash
   RUN_CHARACTERIZATION=1 ./odbc_tests/run_reference.sh -R characterization_{{SNOWFLAKE_TYPE}}_to_{{SQL_C_TYPE}}
   ```
2. Make changes so that the test suite captures driver behaviour
   - Make assertion on all returns and output arguments
   - Make sure the assertions are as strict as possible
   - Make sure we don't use any antipatterns in tests
4. Repeat until the all tests pass

# Plan

## Generate test file
   - Avoid antipatterns
   - Follow guidelines
   - Use the code structure described above
   - Cover all test categories
   - Adjust the tests to reflect driver behaviour - check by running
   ```sh
   RUN_CHARACTERIZATION=1 ./odbc_tests/run_reference.sh -R characterization_{{SNOWFLAKE_TYPE}}_to_{{SQL_C_TYPE}}
   ```

## Iterate until all tests pass and we captured the exact behavour of the driver
   1. Review the test cases and add new if there are gaps in testing the {{SNOWFLAKE_TYPE}} to {{SQL_C_TYPE}} conversion
      - You can go above and beyond predefined test categories
      - You can search the internet for bugs, questions and docs about ODBC conversion
   2. Make sure that all the guidelines are followed and antipatterns avoided
   3. Test the tests using command below and adjust assert, checks and requires as often as possible.
   ```sh
      RUN_CHARACTERIZATION=1 ./odbc_tests/run_reference.sh -R characterization_{{SNOWFLAKE_TYPE}}_to_{{SQL_C_TYPE}}
   ```
   4. Repeat those actions until all the requirements are satisfied and you came up with best characterization test suite possible

# Task
Now generate the tests and make sure you follow guidelines, avoid antipatterns and capture old driver behaviour
