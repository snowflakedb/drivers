# ODBC Driver

## Testing

ODBC tests are written in C++ using CMake and Catch2 framework.

### Prerequisites

Before running tests, ensure you have:
- Set up credentials (see main [README.md](../README.md) for setup instructions)
- CMake 3.10 or later
- C++17 compatible compiler

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
