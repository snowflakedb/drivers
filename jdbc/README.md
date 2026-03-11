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

# Generate old-driver reference coverage (JaCoCo XML + HTML)
./gradlew referenceTest

# Clean and rebuild
./gradlew clean build test
```

### Coverage Streams In CI

- CI runs old-driver reference coverage from `build/reports/jacoco/referenceTest/coverage.xml`.
- CI prints overall line coverage in logs and `GITHUB_STEP_SUMMARY` via `jdbc/ci/reference_tests/extract_coverage.py`.
- JaCoCo artifacts are uploaded as workflow artifacts for inspection.

### Local Coverage Extraction

```bash
python3 ci/reference_tests/extract_coverage.py \
  --report build/reports/jacoco/referenceTest/coverage.xml \
  --label "OLD JDBC reference"
```

### Requirements

- Java 8+
- Gradle 6.0+
- Built Rust components: `sf_core` and `jdbc_bridge`
- Parameters: `parameters.json` (see main [README.md](../README.md) for setup instructions)

### Lombok

`jdbc` uses Lombok in production and test sources via Gradle annotation processors.

If your IDE shows unresolved Lombok symbols, enable annotation processing for the project and refresh Gradle.
