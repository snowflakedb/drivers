---
name: jdbc-test-reviewer
description: >
  Reviews JDBC/Java test code for quality, flakiness, and correctness.
  Use when user says 'review jdbc tests', 'review this jdbc test',
  'is this jdbc test flaky', 'check this java test for flakiness',
  'jdbc test review', or 'review my jdbc test file'.
---

## Opening context

This skill reviews Java test files under `jdbc/src/test/java` for quality, correctness, and flakiness. It maps findings to the project's flaky-test rule catalogue in `.ai/review/universal-driver-flaky-tests.yaml` and the behavioral-difference annotations in `jdbc/BehaviorDifferences.yaml`, then emits a severity-grouped Markdown report with a coverage table and a final checklist.

## Workflow

**Step 1 — Identify files to review.**
If the user names a specific file or passes a path, use that. Otherwise run `git diff --name-only origin/main...HEAD -- 'jdbc/src/test/java/**/*.java'` to collect changed test files on the current branch. If no files are found, say so and stop.

**Step 2 — Load reference material.**
Read `.ai/review/universal-driver-flaky-tests.yaml` to obtain flaky-test rule IDs and descriptions. Read `jdbc/BehaviorDifferences.yaml` to understand which behaviors differ between old and new drivers. Skim any existing `@SkipNewDriver` / `@SkipOldDriver` usages across the test tree with a quick grep so you can cross-check new annotations.

**Step 3 — Categorised quality review.**
For each file, evaluate all of the following categories and collect findings:

- **Resource management** — JDBC objects (`Connection`, `Statement`, `ResultSet`) must be closed in `try-with-resources`. Manual `close()` in `finally` or bare resource opens without cleanup are findings.
- **Test structure** — Verify JUnit 5 annotations (`@Test`, `@BeforeEach`, `@AfterEach`, `@ParameterizedTest`). Distinguish e2e tests (require a live Snowflake endpoint) from unit/WireMock tests. Check Given-When-Then arrangement and `should`-prefixed method names.
- **JDBC call and exception validation** — `assertThrows` must capture `SQLException`; verify that `getSQLState()` and `getErrorCode()` are asserted, not just the message string.
- **Assertions** — No bare `assertTrue(x != null)`; use `assertNotNull`. No magic literals without named constants or explanatory comments.
- **Data retrieval** — Any test fetching multiple rows must have `ORDER BY` in its query. `wasNull()` must be called immediately after the nullable column getter.
- **Behavioral differences** — `@SkipNewDriver` / `@SkipOldDriver` must have a corresponding entry in `jdbc/BehaviorDifferences.yaml`. Missing entries are Medium findings; mismatched skip direction (annotation vs. YAML) are High.
- **Code style / DRY** — Identical setup blocks repeated across test methods should be extracted to `@BeforeEach`. Duplicated SQL strings should be constants.
- **WireMock offline tests** — WireMock stubs must use dynamic ports, not hardcoded ones. `@AfterEach` must call `wireMockServer.resetAll()` or equivalent.
- **Coverage gaps** — Note obvious missing scenarios (null inputs, empty result sets, connection errors).

**Step 4 — Flaky-test pattern detection.**
For each file, scan for the following anti-patterns and map each hit to its rule ID from `universal-driver-flaky-tests.yaml`:

| Anti-pattern | Default severity |
|---|---|
| Bare `Thread.sleep(...)` without timeout comment | High |
| Mutation of shared `getDefaultConnection()` session state | High |
| Hardcoded Snowflake object names (database, schema, warehouse, stage) in multistatement tests | High |
| WireMock on a hardcoded port (non-zero literal in `WireMockServer(...)`) | Medium |
| Missing `@AfterEach` reset of WireMock server | Medium |
| `System.setProperty` / `System.getenv` leak without `@AfterEach` restore | Medium |
| Env-variable dependency without a `@Assumptions.assumeTrue` guard | Low |

**Step 5 — Compile findings and coverage table.**
Group all findings under High / Medium / Low. Build the Missing Test Coverage table from Step 3's gap notes. Write the checklist.

## Output format

Emit a Markdown report in this exact structure for each reviewed file:

```
## jdbc/src/test/java/com/example/FooTest.java

### High
- [ud-no-bare-sleep-for-async-wait] `Thread.sleep(2000)` on line 47 — replace with `Awaitility.await()` or a status-poll loop.

### Medium
- [jdbc-wiremock-must-use-dynamic-port-and-reset] WireMock server created on port 8080 (hardcoded). Use `new WireMockServer(wireMockConfig().dynamicPort())`.

### Low
- Missing `should`-prefix on `testNullColumn` — rename to `shouldReturnNullForNullableColumn`.

---
```

After all per-file sections, append:

```
## Missing Test Coverage

| Area | Missing scenario |
|---|---|
| Error handling | `SQLException` thrown when network drops mid-query |
| wasNull | No test for nullable VARIANT column |

## Checklist

- [ ] All JDBC resources wrapped in try-with-resources
- [ ] All `Thread.sleep` calls removed or justified
- [ ] `@SkipNewDriver`/`@SkipOldDriver` entries present in BehaviorDifferences.yaml
- [ ] WireMock uses dynamic ports and resets in @AfterEach
- [ ] Multi-row queries use ORDER BY
- [ ] wasNull() called immediately after nullable getter
```

## Quality rules

**Pass criteria**

- `try-with-resources` used for every JDBC resource. ✓ `try (ResultSet rs = stmt.executeQuery()) { … }`
- `assertThrows(SQLException.class, …)` used with subsequent `getSQLState()` assertion.
- WireMock port is dynamic: `WireMockServer(wireMockConfig().dynamicPort())`.
- `@SkipNewDriver` / `@SkipOldDriver` has a matching entry in `jdbc/BehaviorDifferences.yaml`.
- No `Thread.sleep` without an inline justification comment and a ticket reference.

**Fail examples**

```java
// ❌ Resource leak
Statement stmt = conn.createStatement();
ResultSet rs = stmt.executeQuery(SQL);

// ❌ Weak exception assertion — message strings change
assertThrows(SQLException.class, () -> stmt.execute(bad));
// missing: assertEquals("22018", e.getSQLState());

// ❌ Hardcoded port
WireMockServer server = new WireMockServer(8080);

// ❌ Shared session mutation
getDefaultConnection().createStatement().execute("ALTER SESSION SET ...");
// test-order dependency: this leaks into every subsequent test
```

Severity escalation rule: any finding that can cause a test to pass on one run and fail on another is automatically High, regardless of the default in the table above.

## Gotchas

- **`getDefaultConnection()` is a shared singleton in many test bases.** Calling `ALTER SESSION` or `USE` on it without teardown is always a High flaky-test finding, even when the test itself looks correct in isolation.
- **`BehaviorDifferences.yaml` may not load if you're offline.** If the file is absent, note it and skip the skip-annotation cross-check rather than halting the review.
- **Multistatement tests are especially prone to hardcoded object names.** The Snowflake multistatement API requires a named warehouse; a hardcoded warehouse name ties the test to a specific account configuration.
- **WireMock `resetAll()` vs `resetMappings()`** — `resetMappings()` does not clear request journal; use `resetAll()` or the finding stands.
- **`@ParameterizedTest` source methods** — check that the source (`@MethodSource`, `@CsvSource`) provides edge-case inputs (null, empty string, zero) not just happy-path values.

## Out of Scope

- **ODBC tests** — for C/C++ test files under `odbc/`, invoke the `odbc-test-reviewer` skill instead.
- **Non-test production code** — this skill does not review `jdbc/src/main/java`. For production code review, use the standard `/review` skill.
- **Performance benchmarking** — latency or throughput analysis is not part of this review.
- **Build system / Maven POM changes** — dependency version decisions are outside this skill's scope.
- **Refactoring suggestions beyond DRY** — the skill flags duplication but does not generate refactored code; it surfaces findings for the author to act on.

## Examples

**Example 1 — Flaky sleep + resource leak**

User: `review this jdbc test`  
File: `StatementTest.java` contains `Thread.sleep(500)` on line 34 and a `ResultSet` opened outside `try-with-resources`.

Output excerpt:
```
## jdbc/src/test/java/.../StatementTest.java

### High
- [ud-no-bare-sleep-for-async-wait] `Thread.sleep(500)` on line 34 — non-deterministic wait; replace with Awaitility or a status-poll loop.
- [ud-no-resource-leak-in-tests] `ResultSet rs` opened on line 22 is not wrapped in try-with-resources; connection close path skips rs.close().
```

**Example 2 — Missing BehaviorDifferences entry**

User: `is this jdbc test flaky`  
File: `ArrowBatchTest.java` has `@SkipNewDriver` on `shouldFetchArrowBatch` but `jdbc/BehaviorDifferences.yaml` has no entry for `ArrowBatchTest#shouldFetchArrowBatch`.

Output excerpt:
```
### High
- @SkipNewDriver on `shouldFetchArrowBatch` (line 61) has no matching entry in jdbc/BehaviorDifferences.yaml. Add an entry or remove the annotation.
```

**Example 3 — Clean file**

User: `jdbc test review`  
File: `ConnectionPoolTest.java` — all resources in try-with-resources, WireMock on dynamic port, `@AfterEach` resets server, no `Thread.sleep`, all assertions use typed matchers, BehaviorDifferences entries present.

Output excerpt:
```
## jdbc/src/test/java/.../ConnectionPoolTest.java

### High
_(none)_

### Medium
_(none)_

### Low
- Consider adding a test for connection acquisition timeout when pool is exhausted.

## Missing Test Coverage
| Area | Missing scenario |
|---|---|
| Pool exhaustion | Timeout path when all connections are in use |
```
