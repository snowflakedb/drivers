# Snowflake JDBC Driver

This is a stub implementation of a JDBC driver for Snowflake that provides the basic JDBC interface and delegates to a native Rust implementation via JNI.

## Testing

- Set up credentials (see main [README.md](../README.md) for setup instructions)
- Java 8+
- Gradle 6.0+

### Running Tests

```bash
export CORE_PATH="$(pwd)/target/debug/libsf_core.dylib"
export PARAMETER_PATH="$(pwd)/parameters.json"
cd jdbc/

# Build and run all tests
./gradlew test

# Run with verbose output
./gradlew test --info

# Run specific test class
./gradlew test --tests SnowflakeDriverTest

# Run specific test method
./gradlew test --tests SnowflakeQueryTest.testSimpleQuery

# Clean and rebuild
./gradlew clean build test
```

### Requirements

- Java 8+
- Gradle 6.0+
- Built Rust components: `sf_core` and `jdbc_bridge`
- Parameters: `parameters.json` (see main [README.md](../README.md) for setup instructions)
