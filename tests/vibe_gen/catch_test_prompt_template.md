# Generate CATCH Tests for ODBC Function: {{FUNCTION_NAME}}

Generate comprehensive C++ CATCH2 tests for the ODBC function `{{FUNCTION_NAME}}`.

## Function Details

**Function Name:** {{FUNCTION_NAME}}

**Return Type:** {{RETURN_TYPE}}

**Parameters:**
{{PARAMETERS}}

## Test Scenarios

The test scenarios are defined in the Gherkin feature file located at:
- `tests/definitions/odbc/vibe/{{FUNCTION_NAME}}/{{FUNCTION_NAME}}.feature`

Read this feature file to understand the test scenarios that need to be implemented.

## Output Location

Generate the CATCH2 test files in:
- `odbc_tests/tests/vibe/{{FUNCTION_NAME}}/`

Create the following files:
1. `test.cpp` - Main test file with CATCH2 test cases
2. `CMakeLists.txt` - CMake configuration for the test

## Reference Implementation

Use the existing tests in `odbc_tests/tests/` as reference for implementation patterns:
- `odbc_tests/tests/basic_tests/test.cpp` - Basic ODBC test patterns
- `odbc_tests/tests/bindings_tests/test.cpp` - Parameter binding tests
- `odbc_tests/tests/auth_tests/` - Authentication tests
- `odbc_tests/tests/datatype_tests/` - Data type handling tests

## Key Files to Reference

1. **Common utilities:**
   - `odbc_tests/common/include/HandleWrapper.hpp` - Handle wrapper classes (EnvironmentHandleWrapper, ConnectionHandleWrapper, StatementHandleWrapper)
   - `odbc_tests/common/include/macros.hpp` - CHECK_ODBC macro for error handling
   - `odbc_tests/common/include/Connection.hpp` - Connection helper class
   - `odbc_tests/common/include/test_setup.hpp` - Test setup utilities

2. **CMakeLists.txt pattern:**
   Follow the pattern from `odbc_tests/tests/bindings_tests/CMakeLists.txt`:
   ```cmake
   cmake_minimum_required(VERSION 4.0)
   add_odbc_test(vibe_{{FUNCTION_NAME}} test.cpp)
   ```

## Test Implementation Guidelines

1. **Test file structure:**
   ```cpp
   #include <catch2/catch_test_macros.hpp>
   #include <sql.h>
   #include <sqlext.h>
   #include <sqltypes.h>
   
   #include "HandleWrapper.hpp"
   #include "Connection.hpp"
   #include "macros.hpp"
   #include "test_setup.hpp"
   
   TEST_CASE("Test description", "[.][vibe][vibe_{{FUNCTION_NAME}}]") {
       // Test implementation
   }
   ```

2. **Tag format:** Use `[vibe_{{FUNCTION_NAME}}]` as the CATCH2 tag for all tests

3. **Error handling:** Use `CHECK_ODBC(ret, handle)` macro to check ODBC return codes

4. **Handle management:** Use the wrapper classes for automatic cleanup:
   - `EnvironmentHandleWrapper` for environment handles
   - `ConnectionHandleWrapper` for connection handles  
   - `StatementHandleWrapper` for statement handles

5. **Connection setup:**
   ```cpp
   Connection conn;  // Creates connected session automatically
   auto stmt = conn.createStatement();
   ```

## Validation

After generating the tests, validate the test suite by running:
```bash
./odbc_tests/run_reference.sh -R vibe_{{FUNCTION_NAME}}
./tests/tests_format_validator/run_validator.sh
```

## Requirements

1. Implement all scenarios from the Gherkin feature file
2. Each Gherkin scenario should map to a TEST_CASE or SECTION
3. Cover happy path, error conditions, and edge cases
4. Use proper ODBC handle management
5. Follow existing code style from reference tests
6. Ensure tests are self-contained and clean up resources properly

## Directory Structure

After generation, the structure should be:
```
odbc_tests/tests/vibe/{{FUNCTION_NAME}}/
├── CMakeLists.txt
└── test.cpp
```

And update `odbc_tests/tests/CMakeLists.txt` to include:
```cmake
add_subdirectory(vibe)
```

If `odbc_tests/tests/vibe/CMakeLists.txt` doesn't exist, create it with:
```cmake
cmake_minimum_required(VERSION 4.0)
add_subdirectory({{FUNCTION_NAME}})
```

# Guidelines
- Search internet for the function usage to better reflect real-world scenarios in the tests
- Please generate tests that can be validate with test_validator - place comments in test code that correspond to lines in .feature file 

