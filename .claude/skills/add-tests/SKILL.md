---
name: add-tests
description: >
  Guides adding tests to the universal driver — decides the right directory,
  whether a Gherkin feature scenario is required, and generates the test stub.
  Use when the user says "add a test", "write a test for", "where should I put this test",
  "add test to JDBC", "add test to Python", "add integ test", "add e2e test",
  or "add unit test".
---

## Overview

Tests in this repo fall into two categories:

1. **Shared behavior** — same behavior expected across multiple drivers; lives in a
   `shared/` Gherkin feature + a matching test file per driver.
2. **Driver-specific behavior** — only one driver implements it; lives in a standalone
   test file with **no** feature entry.

The validator (`tests/tests_format_validator/`) enforces the link between feature
scenarios and test methods. Understanding when it fires — and when it doesn't — is
the core of placing a test correctly.

---

## Step 1 — Determine driver and test level

Ask (or infer from context): which driver, and what kind of test?

| Driver | Unit | Integration | E2E |
|--------|------|-------------|-----|
| Python | `python/tests/unit/` | `python/tests/integ/` | `python/tests/e2e/` |
| JDBC   | `jdbc/src/test/java/net/snowflake/client/` | `jdbc/src/test/java/net/snowflake/jdbc/integration/` | `jdbc/src/test/java/net/snowflake/jdbc/e2e/` |
| ODBC   | `odbc_tests/tests/` (unit) | `odbc_tests/tests/integration/` | `odbc_tests/tests/e2e/` |
| Rust core | `sf_core/tests/` | `sf_core/tests/integration/` | `sf_core/tests/e2e/` |

**Integration vs E2E distinction (JDBC)**
- `jdbc/integration/` = WireMock-based; stubs the server, no live Snowflake needed.
- `jdbc/e2e/` = live Snowflake account required.
- A test that calls `DriverManager.getConnection` against a real account is **always** E2E.

---

## Step 2 — Decide whether a Gherkin feature is needed

The validator fires on a file **only** when the file's name matches an existing
feature filename (normalized: `FooBarTests.java` ↔ `foo_bar`, `test_foo_bar.py` ↔ `foo_bar`).

### Shared behavior → feature required

If the behavior is (or will be) common across two or more drivers:

1. Check `tests/definitions/shared/<category>/` for an existing feature that covers it.
   - Found → add a `@{driver}_e2e` scenario to that feature, then add the matching test method.
   - Not found → create `shared/<category>/<name>.feature`.
2. Place the test in the matching directory (same filename stem as the feature):
   - JDBC e2e: `jdbc/e2e/<category>/<FeatureName>Tests.java`
   - Python e2e: `python/tests/e2e/<category>/test_<feature_name>.py`
3. Feature files must live under `shared/` — language-specific directories
   (`jdbc/`, `python/`, etc.) are **not allowed** under `tests/definitions/`.

### Driver-specific behavior → no feature needed

If the test covers something only one driver does:

1. Create a **new file** whose name does **not** match any existing feature file.
   The validator skips files with no matching feature. Confirmed by running the validator
   locally: `bash tests/tests_format_validator/run_validator.sh`.
2. Keep `// Given / When / Then` (or `# Given / When / Then`) Gherkin comments
   inside the method for readability — they don't need a backing feature file.
3. Place the file in the appropriate `e2e/<category>/` directory.

Real examples already in the repo (no feature backing):
- `python/tests/e2e/query/test_connection_properties.py`
- `python/tests/e2e/query/test_parameter_binding_python.py`
- `jdbc/src/test/java/net/snowflake/jdbc/e2e/session/ClientOptionsTests.java`

> **Rule of thumb**: "should other drivers also test this?" If yes → shared feature.
> If no → standalone file with a fresh name.

---

## Step 3 — Naming conventions

### Feature scenarios
- Scenario names use "should" statements: `should accept X when Y`
- Scenario name → test method: normalize spaces to camelCase (JDBC) or `test_`+snake_case (Python):
  `should accept X` → `shouldAcceptX` / `test_should_accept_x`
- The validator matches these by normalized comparison — casing and separators are ignored.

### JDBC test methods
- `@Test` names are `should`-prefixed camelCase: `shouldReturnDefaultThreadCount`
- Assertions: typed — `assertEquals(4, conn.getCount())`, not `assertEquals(true, ...)`
- All JDBC resources (`Connection`, `Statement`, `ResultSet`) in `try-with-resources`

### Python test methods
- `test_should_`-prefixed snake_case: `test_should_return_default_value`
- Unit tests use `mock_db_api` fixture from `python/tests/helpers/fixtures.py`

---

## Step 4 — Gherkin step comments (when file matches a feature)

The validator requires each `@Test` / `def test_` method in a tracked file to have
at least one `When` and one `Then` step comment, each followed by implementation code.

**JDBC:**
```java
@Test
void shouldEnableSessionKeepAliveViaConnectionString() throws Exception {
    // Given Snowflake client is logged in with CLIENT_SESSION_KEEP_ALIVE set to "true"
    Properties props = withDefaultAuth(loadDefaultConnectionProperties());
    props.setProperty("CLIENT_SESSION_KEEP_ALIVE", "true");
    try (Connection conn = DriverManager.getConnection(buildJdbcUrl(props), props);
        Statement stmt = conn.createStatement();
        // When Query "SHOW PARAMETERS LIKE 'CLIENT_SESSION_KEEP_ALIVE'" is executed
        ResultSet rs = stmt.executeQuery("SHOW PARAMETERS LIKE 'CLIENT_SESSION_KEEP_ALIVE'")) {
      // Then the session parameter value should be "true"
      assertEquals("true", rs.getString("value"));
    }
}
```

**Python:**
```python
def test_should_return_configured_value(self, connection_factory):
    # Given connection is established with client_prefetch_threads=8
    with connection_factory(client_prefetch_threads=8) as conn:
        # When client_prefetch_threads is read
        # Then it should equal 8
        assert conn.client_prefetch_threads == 8
```

---

## Step 5 — Feature file tags

```gherkin
@core @python @jdbc          ← feature-level: which drivers own this feature
Feature: Session parameters via connection options

  @jdbc_e2e @python_e2e      ← scenario: driver + level (_e2e, _int, _unit)
  Scenario: should enable session keep-alive via connection string
    Given ...
    When ...
    Then ...

  @jdbc_e2e                  ← single-driver scenario; others see "TODO" in coverage
  Scenario: should set heartbeat frequency via connection string
    ...
```

To explicitly exclude a driver: `@jdbc_not_needed` on the feature or scenario.

---

## Step 6 — Verify locally

```bash
bash tests/tests_format_validator/run_validator.sh
```

Catches: orphaned methods, missing When/Then comments, feature files outside `shared/`,
and mismatched scenario-to-method names.

---

## Decision tree

```
Is this behavior shared across ≥2 drivers?
├── Yes → needs a shared feature
│   ├── Existing feature covers it? → add @{driver}_e2e scenario + matching test method
│   └── No → create tests/definitions/shared/<category>/<name>.feature
│
└── No → driver-specific, no feature needed
    ├── Needs live Snowflake? → e2e/ dir, new file with a name that matches no feature
    ├── WireMock only? (JDBC) → jdbc/integration/
    └── No server? → python/tests/unit/ or jdbc/src/test/.../client/
```

---

## Common mistakes

| Mistake | Consequence | Fix |
|---------|-------------|-----|
| Adding a method to a feature-matched file without adding a scenario | Validator flags orphaned method | Add the scenario, or move method to a new non-matching file |
| Creating `tests/definitions/jdbc/` for a JDBC-specific test | Not allowed — validator enforces `shared/` only | Use a standalone file instead (no feature needed) |
| Live-server test placed in `jdbc/integration/` | Wrong tree — `integration/` is WireMock-only | Move to `jdbc/e2e/` |
| Using `SHOW PARAMETERS LIKE 'CLIENT_PREFETCH_THREADS'` to verify a client-side hint | Server returns default (4) regardless | Use `SELECT 1` to verify the connection accepts the property |
