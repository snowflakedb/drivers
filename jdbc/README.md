# Snowflake JDBC Driver

This is a stub implementation of a JDBC driver for Snowflake that provides the basic JDBC interface and delegates to a native Rust implementation via JNI.

## Consuming the driver

The driver is published under `net.snowflake` in three flavors:

| Coordinate | What it is |
|---|---|
| `net.snowflake:snowflake-jdbc-native:<version>:<classifier>` | **Recommended.** Self-contained *per-platform* jar: thin Java + that platform's native, in one artifact. Dependencies (Arrow, protobuf, …) resolve from the POM. One dependency, one platform. |
| `net.snowflake:snowflake-jdbc-native-all` | Self-contained *fat* jar: all dependencies shaded **and** every platform's native bundled in. Portable across platforms in a single artifact; larger, and shades transitive deps. |
| `net.snowflake:snowflake-jdbc-native` (no classifier) | **Advanced / native-free.** No native bundled — the JVM resolves the lib from `CORE_PATH` or the `jdbc.library.path` system property. Used on unlisted platforms or when you supply the native out-of-band. Not runnable on its own. |

The `<classifier>` follows the [`os-maven-plugin`](https://github.com/trustin/os-maven-plugin) / [`osdetector`](https://github.com/google/osdetector-gradle-plugin) convention: `linux-x86_64`, `linux-aarch_64`, `osx-x86_64`, `osx-aarch_64`, `windows-x86_64`.

> **Note on the classifier-less coordinate.** A bare `net.snowflake:snowflake-jdbc-native:<version>`
> (no classifier) *resolves* fine but is native-free, so it fails at first use with a signpost
> pointing back here. Maven and Gradle cannot auto-substitute the platform classifier from
> module metadata alone, so pick a classifier explicitly (below) or use `-all`.

### Per-platform classifier jar — Maven (recommended)

Add the `os-maven-plugin` extension so `${os.detected.classifier}` fills in automatically:

```xml
<build><extensions>
  <extension>
    <groupId>kr.motd.maven</groupId><artifactId>os-maven-plugin</artifactId><version>1.7.1</version>
  </extension>
</extensions></build>

<dependencies>
  <dependency>
    <groupId>net.snowflake</groupId><artifactId>snowflake-jdbc-native</artifactId>
    <version>0.0.1</version><classifier>${os.detected.classifier}</classifier>
  </dependency>
</dependencies>
```

(Or hard-code the classifier, e.g. `<classifier>osx-aarch_64</classifier>`, when you build for a fixed target.)

### Per-platform classifier jar — Gradle (recommended)

Apply the `osdetector` plugin so `osdetector.classifier` fills in the running platform:

```groovy
plugins { id 'com.google.osdetector' version '1.7.3' }

dependencies {
    implementation "net.snowflake:snowflake-jdbc-native:0.0.1:${osdetector.classifier}"
}
```

(Or hard-code the classifier, e.g. `...:0.0.1:osx-aarch_64`, when you build for a fixed target.)

### Self-contained fat jar (portable, all platforms in one)

```groovy
implementation 'net.snowflake:snowflake-jdbc-native-all:0.0.1'
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
