# Snowflake JDBC Driver

This is a stub implementation of a JDBC driver for Snowflake that provides the basic JDBC interface and delegates to a native Rust implementation via JNI.

## Consuming the driver

The driver is published under `net.snowflake` in two flavors:

| Coordinate | What it is |
|---|---|
| `net.snowflake:snowflake-jdbc-v2` | **Thin** (recommended): unshaded jar; dependencies (Arrow, protobuf, …) resolve from Maven. Needs a matching `-native` artifact for your platform. |
| `net.snowflake:snowflake-jdbc-v2-standalone` | **Self-contained**: one shaded jar with all dependencies **and** every platform's native bundled in. No `-native` artifact needed. |
| `net.snowflake:snowflake-jdbc-v2-native:<classifier>` | The compiled native library for **one** platform. Paired with the thin jar. |

The native `<classifier>` follows the [`os-maven-plugin`](https://github.com/trustin/os-maven-plugin) / [`osdetector`](https://github.com/google/osdetector-gradle-plugin) convention: `linux-x86_64`, `linux-aarch_64`, `osx-x86_64`, `osx-aarch_64`, `windows-x86_64`.

### Thin jar + native (Maven)

Add the `os-maven-plugin` extension so `${os.detected.classifier}` fills in automatically:

```xml
<build><extensions>
  <extension>
    <groupId>kr.motd.maven</groupId><artifactId>os-maven-plugin</artifactId><version>1.7.1</version>
  </extension>
</extensions></build>

<dependencies>
  <dependency>
    <groupId>net.snowflake</groupId><artifactId>snowflake-jdbc-v2</artifactId><version>4.0.0</version>
  </dependency>
  <dependency>
    <groupId>net.snowflake</groupId><artifactId>snowflake-jdbc-v2-native</artifactId>
    <version>4.0.0</version><classifier>${os.detected.classifier}</classifier>
  </dependency>
</dependencies>
```

### Thin jar + native (Gradle)

Apply the `osdetector` plugin so `osdetector.classifier` fills in the running platform:

```groovy
plugins { id 'com.google.osdetector' version '1.7.3' }

dependencies {
    implementation 'net.snowflake:snowflake-jdbc-v2:4.0.0'
    runtimeOnly "net.snowflake:snowflake-jdbc-v2-native:4.0.0:${osdetector.classifier}"
}
```

(Or hard-code the classifier, e.g. `...:4.0.0:osx-aarch_64`, when you build for a fixed target.)

### Self-contained jar (no native artifact)

```groovy
implementation 'net.snowflake:snowflake-jdbc-v2-standalone:4.0.0'
```

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
