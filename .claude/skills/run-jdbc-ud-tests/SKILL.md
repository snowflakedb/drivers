---
name: run-jdbc-ud-tests
description: >
  Runbook for building and running JDBC Universal Driver (UD) tests. Use
  when you need to compile the Rust jdbc_bridge, set up CORE_PATH, and
  execute JDBC UD tests via Gradle.
---

## JDBC UD Test Runner

All commands run from the **repo root** unless stated otherwise.

---

## Prerequisites

- Java 8+ (Java 21 recommended for WireMock tests; HotSpot JVM — **not OpenJ9**)
- Gradle (bundled as `jdbc/gradlew` — no global install needed)
- Rust toolchain with `cargo` on PATH

---

## Step 1 — Credentials

Ensure `parameters.json` exists at the repo root and `PARAMETER_PATH` is
exported. Full setup procedure:

@.claude/rules/ud-credentials.md

---

## Step 2 — Build the Rust bridge

The JDBC driver loads a JNI bridge library (`jdbc_bridge`) which links
`sf_core` statically. **Tests fail with `UnsatisfiedLinkError` if absent.**

```bash
# Debug build (faster, used for local dev):
cargo build --package jdbc_bridge
# Output: target/debug/libjdbc_bridge.dylib  (macOS)
#         target/debug/libjdbc_bridge.so      (Linux)
#         target/debug/jdbc_bridge.dll        (Windows)

# Release build:
cargo build --release --package jdbc_bridge
```

Export the path so Gradle can find it:

```bash
# macOS:
export CORE_PATH="$(pwd)/target/debug/libjdbc_bridge.dylib"
# Linux:
export CORE_PATH="$(pwd)/target/debug/libjdbc_bridge.so"
# Windows:
# set CORE_PATH=%cd%\target\debug\jdbc_bridge.dll
```

---

## Step 3 — Run tests

```bash
cd jdbc
```

### All tests

```bash
./gradlew test
```

### Specific test class

```bash
./gradlew test --tests SnowflakeDriverTest
```

### Specific test method

```bash
./gradlew test --tests SnowflakeQueryTest.testSimpleQuery
```

### With verbose output

```bash
./gradlew test --info
```

### Reference tests (old driver 4.0.1 compatibility)

```bash
./gradlew referenceTest jacocoReferenceReport
```

### Parity tests (DateTime comparison between new and old driver)

```bash
./gradlew parityTest
# Note: requires HotSpot JVM — OpenJ9 causes stack overflow
```

### Coverage report

```bash
./gradlew test jacocoTestReport
# Report: jdbc/build/reports/jacoco/test/html/
```

---

## Key environment variables

| Variable | Required | Purpose |
|---|---|---|
| `PARAMETER_PATH` | Yes | Path to `parameters.json` credentials |
| `CORE_PATH` | Yes | Path to compiled `libjdbc_bridge.{so,dylib,dll}` |
| `QUERY_RESULT_FORMAT` | No | Set `JSON` to test JSON result format |
| `GRADLE_TEST_RETRY_COUNT` | No | Retries per failing test (e.g. `2`) |
| `GRADLE_INCLUDE_TAGS` | No | Comma-separated tags to include (e.g. `requires_browser`) |
| `GRADLE_EXCLUDE_TAGS` | No | Comma-separated tags to exclude (default: `requires_browser`) |

---

## Troubleshooting

### `UnsatisfiedLinkError` at test startup

`CORE_PATH` is unset or points to a missing file. Build the bridge and
export the path (see Step 2).

### Rebuild after Rust changes

```bash
cargo build --package jdbc_bridge
export CORE_PATH="$(pwd)/target/debug/libjdbc_bridge.dylib"   # or .so
cd jdbc && ./gradlew test
```

### Clean Gradle build

```bash
cd jdbc && ./gradlew clean test
```

### Parity tests fail on OpenJ9 JVM

Switch to a HotSpot JVM (Temurin/Oracle). OpenJ9 is not supported for
parity tests due to stack overflow behaviour.
