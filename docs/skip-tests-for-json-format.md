## Skipping Tests for JSON Result Format

Some tests require Arrow-specific precision that JSON format cannot provide (e.g., IEEE 754 boundary values like `Double.MAX_VALUE = 1.7976931348623157e308`). JSON serialization truncates these values to ~15 significant digits, causing precision loss.

This document describes how to mark tests to skip when `QUERY_RESULT_FORMAT=JSON` is set.

## Environment Variable

All wrappers use the unified environment variable:
```bash
export QUERY_RESULT_FORMAT=JSON
```

## Python (pytest)

### Setup

The `@pytest.mark.skip_for_json_result_set` marker is already configured in:
- **Marker registration**: `python/pyproject.toml` under `[tool.pytest.ini_options].markers`
- **Hook implementation**: `python/tests/conftest.py` with `pytest_runtest_setup()` hook

### Usage

```python
import pytest

@pytest.mark.skip_for_json_result_set(reason="JSON format loses precision for Float.MAX_VALUE")
def test_boundary_values(execute_query):
    # Test that requires Arrow precision
    result = execute_query("SELECT 1.7976931348623157e308::FLOAT")
    assert result[0] == 1.7976931348623157e308
```

### Example

See: `python/tests/e2e/types/test_float.py`

## Java/JDBC (JUnit 5)

### Setup

The `@SkipForJSONResultSet` annotation is already configured:
- **Annotation**: `jdbc/src/test/java/net/snowflake/client/SkipForJSONResultSet.java`
- **Extension**: `jdbc/src/test/java/net/snowflake/client/SkipForJSONResultSetCondition.java`

### Usage

```java
import net.snowflake.client.SkipForJSONResultSet;
import org.junit.jupiter.api.Test;

@Test
@SkipForJSONResultSet("JSON format loses precision for Double.MAX_VALUE")
public void testBoundaryValues() throws Exception {
    // Test that requires Arrow precision
    ResultSet rs = stmt.executeQuery("SELECT 1.7976931348623157e308::FLOAT");
    assertTrue(rs.next());
    assertEquals(Double.MAX_VALUE, rs.getDouble(1));
}
```

### Example

See: `jdbc/src/test/java/net/snowflake/jdbc/e2e/types/FloatTests.java`

## C++/ODBC (Catch2)

### Setup

The `SKIP_FOR_JSON_RESULT_SET` macro is defined in:
- **Header**: `odbc_tests/common/include/test_setup.hpp`

### Usage

```cpp
#include "test_setup.hpp"
#include <catch2/catch_test_macros.hpp>

TEST_CASE("boundary value test", "[float]") {
  SKIP_FOR_JSON_RESULT_SET("JSON format loses precision for Double.MAX boundary values");
  
  // Test that requires Arrow precision
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT 1.7976931348623157e308::FLOAT");
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 1) == Catch::Approx(1.7976931348623157e308));
}
```

### Example

See: `odbc_tests/tests/e2e/types/float.cpp`

## When to Use

Use these skip mechanisms for tests that:

1. **Test boundary values** of floating-point types (e.g., `Double.MAX_VALUE`, `Double.MIN_NORMAL`)
2. **Require exact precision** beyond ~15 significant digits
3. **Compare binary representations** that JSON cannot preserve
4. **Test Arrow-specific features** not supported in JSON format

## Testing Locally

```bash
# Run tests with JSON format
export QUERY_RESULT_FORMAT=JSON

# Python
cd python && hatch run test

# JDBC
cd jdbc && ./gradlew test

# ODBC
./scripts/odbc/run_tests_unix.sh
```

## Why Tests Fail with JSON

JSON serialization of floating-point numbers is limited to ~15-16 significant digits. When `Double.MAX_VALUE = 1.7976931348623157e308` (17 digits) is serialized to JSON as `"1.79769313486232e+308"` (15 digits), it rounds to `Infinity` when parsed back.
