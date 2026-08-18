---
name: test-coverage-mapper
description: >
  Maps old driver test files and test case names (ODBC/JDBC/Python) to Universal Driver equivalents in tests/oldTestsCoverage/ YAML files, and verifies coverage via assertion-level code analysis. Use when the user says: 'map old tests', 'map old ODBC/JDBC/Python test', 'which old tests are unmapped', 'coverage gaps for ODBC/JDBC/Python', 'add test mapping', 'add a test mapping to odbc.yaml', 'reverse lookup', 'sync test mappings', 'which UD tests cover this old test', 'update the coverage yaml', 'reconcile summary counts in the yaml', 'TC_AUTH_NNN shows unmapped'. NOT for Jira bug tickets (use bug-regression-coverage for SNOW-XXXXXX). NOT for code coverage metrics such as line coverage, branch coverage, jacoco reports, coverage percentages, or coverage of newly written features.
---

## Opening context

This skill orchestrates the workflow for mapping legacy driver tests (ODBC, JDBC, Python) to their Universal Driver equivalents, updating YAML mapping files in `tests/oldTestsCoverage/`, and verifying coverage equivalence via assertion-level code analysis. It is the primary entry point whenever a developer needs to understand, extend, or audit the cross-driver test coverage relationship.

## Workflow

Work proceeds in up to seven sequential steps; the active mode (determined in Step 0) controls which steps execute and how far they go.

### Step 0 — Determine intent and scope

Identify which of the four task modes the user is requesting:

| Mode | Key phrases |
|---|---|
| **Add mapping** | "map old tests", "add test mapping", "which old tests are missing" |
| **Coverage gap report** | "coverage gap", "compare old and new tests", "missing coverage" |
| **Reverse lookup** | "reverse lookup UD test", "which old tests does X cover" |
| **Sync test lists** | "sync test mappings", "update the yaml" |

If the driver is not specified, default to **ODBC** (pilot set, 432 tests). State: `"No driver specified — defaulting to ODBC. Specify jdbc or python to override."`

---

### Step 0b — Resolve old driver source (mandatory)

Before any analysis that requires reading test source code (Steps 2–6), **ask the user** for the old driver repo location. Do not proceed without this.

Prompt:
> "I need access to the old [DRIVER] test source code to analyze assertions. Please provide either:
> 1. A local path to the repo (e.g., `/Users/you/snowflake-odbc`)
> 2. The GitHub repo path (e.g., `snowflakedb/snowflake-odbc`) — I'll use GitHub MCP to fetch files
>
> Which do you have?"

**Resolution logic:**
- **Local path provided** → verify it exists, use `Read` tool to access test files directly
- **GitHub repo provided** → use `mcp__github__github_get_file` to fetch test files on demand
- **Neither provided** → block progression. State: `"Cannot analyze test assertions without access to the old driver source. Please provide a path."`

Store the resolved source reference for use in subsequent steps. Known repos
and typical local paths are in:

@.claude/rules/driver-repos.md

---

### Step 1 — Load and inspect the current mapping file

Read the relevant YAML from `tests/oldTestsCoverage/{odbc,jdbc,python}.yaml`. The file structure is:

```yaml
title: ODBC Driver Test Coverage
description: Mapping of old ODBC driver tests to new Universal Driver test suite.
summary:
  total_tests: 432
  total_test_files: 71
tests:
  AuthenticationTests/AuthLatestTest/ExternalBrowserTest.cpp:   # old test file path
  - test_name: should authenticate using external browser
    ud_tests:                          # list of UD tests covering this (many-to-many)
      - path: tests/e2e/auth/oauth_test.rs::test_oauth_basic
      - path: tests/unit/auth/token_exchange_test.rs::test_exchange_flow
    status: mapped                     # unmapped | partial | mapped | not-applicable
    notes: "SNOW-3548054: browser launch + token exchange"
    jira: "SNOW-3548054"               # optional ticket reference
  - test_name: should throw error for wrong okta credentials
    ud_tests:
      - path: sf_core/tests/e2e/authentication/native_okta.rs - vpn_should_fail
    status: partial                    # some assertions covered, gaps documented below
    gaps:                              # required when status is 'partial'
      - "No UD test verifies SQLSTATE 28000 error code on auth failure"
      - "Recovery path (retry with correct creds) not exercised by any UD test"
    notes: "Assertions 1-2 covered (error type, message). 2 gaps remain."
    jira: "SNOW-3548054"
  - test_name: should throw error for browser timeout
    ud_tests: []                       # empty = unmapped
    status: unmapped
```

**Status values:**

| Status | Meaning | `ud_tests` | `gaps` field |
|---|---|---|---|
| `unmapped` | No UD equivalent identified yet | empty `[]` | not required |
| `partial` | Some assertions covered, others missing | populated | **required** — list each uncovered assertion |
| `mapped` | All assertions covered by UD tests (confirmed via code analysis) | populated | not required |
| `not-applicable` | Old test is deprecated/driver-specific, intentionally not ported | empty or populated | not required |

**Key design decisions:**
- Mappings are **many-to-many**: one old test maps to multiple UD tests (list under `ud_tests`), and one UD test can appear under multiple old-test entries across the file.
- **`partial` vs `mapped`**: use `partial` when the assertion-level analysis (Step 3b) finds gaps. Use `mapped` only when ALL old-test assertions are accounted for in the mapped UD tests. This distinction is critical — it surfaces exactly where new UD tests are needed.
- The `gaps` field is a list of strings, each describing one uncovered assertion or behavior in plain language. It is **required** when `status: partial` and forbidden when `status: mapped`.
- The agent's ceiling is `status: mapped`. Promotion beyond that (e.g., to `verified` via CI execution) is out of scope for this skill.
- If the YAML file still uses the legacy format (bare `ud_tests: []` without `status` fields), treat empty arrays as `unmapped` and non-empty arrays as `mapped`. Offer to migrate entries to the full schema on first edit.

---

### Step 1b — Mandatory search checklist (BLOCKING — must complete before Step 4)

For every old test being mapped, you must search ALL locations below and **report results for each in your output**. This checklist is not optional — if any row is missing from your output, the mapping is invalid.

**Output this table for every mapping** (fill in "found" or "no match" for each):

```
SEARCH CHECKLIST for: "<old test name>"
| # | Location                              | Searched? | Result                          |
|---|---------------------------------------|-----------|----------------------------------|
| 1 | sf_core/tests/integration/            | YES/NO    | <file - function> or "no match" |
| 2 | sf_core/tests/e2e/                    | YES/NO    | <file - function> or "no match" |
| 3 | sf_core/src/ (grep #[cfg(test)])      | YES/NO    | <file - function> or "no match" |
| 4 | python/tests/unit/                    | YES/NO    | <file - function> or "no match" |
| 5 | python/tests/integ/                   | YES/NO    | <file - function> or "no match" |
| 6 | python/tests/e2e/                     | YES/NO    | <file - function> or "no match" |
| 7 | odbc_tests/tests/e2e/                 | YES/NO    | <file - function> or "no match" |
| 8 | odbc_tests/tests/integration/         | YES/NO    | <file - function> or "no match" |
| 9 | odbc_tests/tests/odbc-api/            | YES/NO    | <file - function> or "no match" |
|10 | odbc_tests/tests/basic_tests/         | YES/NO    | <file - function> or "no match" |
|11 | odbc_tests/tests/bindings_tests/      | YES/NO    | <file - function> or "no match" |
|12 | nodejs/tests/unit/                    | YES/NO    | <file - function> or "no match" |
|13 | nodejs/tests/e2e/                     | YES/NO    | <file - function> or "no match" |
|14 | tests/definitions/shared/             | YES/NO    | <scenario name> or "no match"   |
|15 | jdbc/src/test/java/                   | YES/NO    | <method> or "no match"          |
```

**Rules:**
- Every row must say YES in "Searched?" column. A "NO" makes the mapping INVALID.
- For each YES, you must have actually run a Grep or Read on that directory. Just stating "no match" without searching is a violation.
- "no match" is a valid result — not every old test will have coverage in every location. But you must prove you looked.
- When you find a match, READ THE FULL TEST BODY before adding it to ud_tests. A file path match alone is not sufficient.

**What to search for in each location:**
- Use keywords from the old test's assertion inventory (error types, function names, config keys, API endpoints)
- Search by behavior: "authentication", "timeout", "retry", "password", "error" etc.
- For `sf_core/src/` inline tests — **do NOT satisfy with a single wide grep**. Follow this four-step discovery process:
  1. **Extract keywords** from the old test's assertion inventory: function names, config key names, auth method names, error type names, API endpoint patterns (e.g. `password`, `user_password`, `authenticator`, `connection_config`, `session`).
  2. **Find relevant source files**: for each keyword, run `grep -rl "<keyword>" sf_core/src/ --include="*.rs"` — collect every file that mentions the functionality under analysis.
  3. **Filter to files with inline tests**: for each discovered file, run `grep -c "#\[test\]" <file>` — keep only files where the count is >0.
  4. **Read every test function** in the filtered files: open the file, find all `#[test]`-annotated functions, read their bodies, and check whether any assertion matches the behavior being mapped.
  Report result as: "Searched N source files matching keywords, M contained inline tests, K were relevant" — or "Searched N source files, 0 contained inline tests matching <keyword>."
  "no match" is only valid when you show which files were grepped and confirmed to have 0 relevant `#[test]` functions. Stating "no match" without showing grep output is a **violation**.

**Why this checklist exists:** In testing, agents skipped `sf_core/tests/integration/` and `python/tests/unit/` — missing tests that directly covered gaps. The checklist makes skipping structurally impossible.

#### Directory reference (what each location contains)

| Location | Language | Framework | What lives there |
|---|---|---|---|
| `sf_core/tests/integration/` | Rust | `#[test]` | Auth, config, HTTP, query, session — wiremock + real integration |
| `sf_core/tests/e2e/` | Rust | `#[test]` | Full end-to-end with real Snowflake (VPN required) |
| `sf_core/src/**/` | Rust | `#[cfg(test)]` | Inline unit tests in source modules (1059 tests!) |
| `python/tests/unit/` | Python | pytest | Mocked unit tests — connection, cursor, config, binding |
| `python/tests/integ/` | Python | pytest | Integration with mocked/real server |
| `python/tests/e2e/` | Python | pytest | Full e2e with real Snowflake |
| `odbc_tests/tests/e2e/` | C++ | Catch2 | ODBC e2e tests |
| `odbc_tests/tests/integration/` | C++ | Catch2 | ODBC integration tests |
| `odbc_tests/tests/odbc-api/` | C++ | Catch2 | Low-level ODBC API function tests |
| `odbc_tests/tests/basic_tests/` | C++ | Catch2 | Basic ODBC functionality unit tests |
| `odbc_tests/tests/bindings_tests/` | C++ | Catch2 | ODBC bindings unit tests |
| `nodejs/tests/unit/` | TypeScript | Vitest | Node.js unit tests — mocked behavior |
| `nodejs/tests/e2e/` | TypeScript | Vitest | Node.js e2e tests with real Snowflake |
| `tests/definitions/shared/` | Gherkin | BDD | Scenario definitions (intent only, not assertions) |
| `jdbc/src/test/java/` | Java | JUnit | JDBC test implementations |

#### Behavior differences files (per wrapper)

When a gap involves a deliberate behavioral change between old and UD, check the relevant file before writing the gap:

| Driver | File |
|---|---|
| ODBC | `odbc_tests/BehaviorDifferences.yaml` |
| Python | `python/BehaviorDifferences.yaml` |
| JDBC | `jdbc/BehaviorDifferences.yaml` |
| Node.js | `nodejs/BehaviorDifferences.yaml` |
---

### Step 2 — Deep analysis of old test (Add mapping and Coverage gap modes)

For each unmapped or ambiguous old test, perform a **full body analysis** — not just name matching:

#### 2a. Read the full test source code

Use GitHub MCP (`github_get_file`) to fetch the old test file. For each test function, extract:

- **Setup/preconditions**: what state is established before the action (connections, configs, fixtures)
- **Action under test**: the specific API call or operation being exercised
- **Assertions**: every `REQUIRE`, `ASSERT`, `assertEqual`, `assert`, `CHECK` — the complete list of verified outcomes
- **Edge cases covered**: error paths, boundary values, timeouts, retries
- **Teardown/cleanup**: any postcondition verification

#### 2b. Build an assertion inventory

Create a structured list of what the test actually verifies:

```
Test: "should throw error for wrong okta credentials"
Assertions:
  1. Connection attempt raises AuthenticationError (not generic error)
  2. Error message contains "incorrect username or password"
  3. Error code matches SQLSTATE 28000
  4. No token is cached after failure
  5. Retry with correct credentials succeeds (recovery path)
Scope:
  - Exercises: REST /session/authenticator-request endpoint
  - Covers: 401 + 403 HTTP responses from Okta
  - Does NOT cover: network timeout, malformed response, expired token
```

#### 2c. Gather context from commit history and Jira

1. **GitHub MCP**: look up the test's commit history. Read the commit messages and PR descriptions that introduced or last modified the test.
2. **Jira MCP**: if the commit message references a ticket (e.g. `SNOW-XXXXXX`), fetch the ticket summary and acceptance criteria.
3. Cross-reference: does the ticket mention assertions or scenarios not visible in the current test code? (Tests evolve — the original intent may be broader than what's currently implemented.)

#### 2d. Synthesize a detailed coverage statement

Produce a structured intent + coverage statement (not just one sentence):

```
Intent: Verifies that authentication fails gracefully with wrong Okta credentials.
Coverage scope:
  - Error type discrimination (AuthError vs generic)
  - Error message content for user-facing diagnostics
  - SQLSTATE compliance
  - Cache hygiene after failure
  - Recovery path (retry succeeds)
Source: SNOW-3548054 AC items 2-4; assertion 5 added in commit abc1234.
```

This detailed analysis is critical — two tests may have similar names but different assertion scopes. A mapping is only valid when ALL assertions in the old test are covered by the mapped UD test(s).

---

### Step 3 — Find and validate candidate UD tests (Add mapping mode)

Given the assertion inventory from Step 2, locate UD tests that cover the **same assertions and scope** — not just the same intent:

#### 3a. Search for candidates — read ACTUAL TEST CODE, not just definitions

**CRITICAL**: Feature definitions (`tests/definitions/shared/*.feature`) describe WHAT should be tested in Gherkin, but the actual assertions live in the implementation files. You MUST read the implementation code to verify coverage.

**Search order (mandatory — do not skip any step):**

1. **Rust core tests** — these are the primary UD implementation and most likely to cover the behavior:
   - `sf_core/tests/integration/` — read the `.rs` files, look at `assert!`, `assert_eq!`, `#[should_panic]`, error matching
   - `sf_core/tests/e2e/` — read the `.rs` files, look at actual Snowflake calls and result assertions
   - `sf_core/src/` inline unit tests — 1059+ tests co-located with production code in `.rs` source files (NOT in `mod.rs`). Use the four-step discovery process from Step 1b row 3: extract keywords from the assertion inventory → grep `sf_core/src/` to find source files covering the functionality → filter to files with `#[test]` functions → read every matching test body.

2. **ODBC C++ tests** (for ODBC mappings):
   - `odbc_tests/tests/e2e/` — read `.cpp` files, look at `REQUIRE()`, `CHECK()`, `SECTION()` blocks
   - `odbc_tests/tests/integration/` — same
   - `odbc_tests/tests/odbc-api/` — low-level ODBC API function tests
   - `odbc_tests/tests/basic_tests/` — basic ODBC functionality unit tests
   - `odbc_tests/tests/bindings_tests/` — ODBC bindings unit tests

3. **Python tests** (for Python mappings):
   - `python/tests/unit/` — read test files, look at `assert`, `pytest.raises`, mocked behavior
   - `python/tests/integ/` — integration tests with real or mocked server
   - `python/tests/e2e/` — full end-to-end with actual Snowflake

4. **Feature definitions** (supplementary only — not sufficient alone):
   - `tests/definitions/shared/` — Gherkin scenarios describe intent but NOT assertion details
   - A feature scenario match is a **starting point** — you must then find and read the step implementation

5. **JDBC/Node.js** (for respective mappings):
   - `jdbc/src/test/java/` — JUnit assertions
   - `nodejs/tests/unit/` — Vitest unit tests, mocked behavior
   - `nodejs/tests/e2e/` — Vitest e2e with real Snowflake

**For each candidate found, you MUST read the full test body.** A file path match or test name match is NOT sufficient. Open the file, read the function, extract every assertion. Example:

```rust
// sf_core/tests/integration/authentication/user_password.rs
#[test]
fn should_fail_authentication_when_user_is_not_provided() {
    let config = ConnectionConfig::builder()
        .account("testaccount")
        .password("pass")
        // NOTE: no .user() — this is the empty-username case
        .build();
    let result = Connection::connect(&config);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, SnowflakeError::Auth(_)));  // assertion 1: error type
    assert!(err.to_string().contains("user"));        // assertion 2: message content
}
```

This test directly covers "empty username raises error" — it's a unit/integration test in Rust, NOT visible in feature definitions. If you only checked `tests/definitions/`, you'd miss it entirely.

#### 3b. Build a comparison matrix

For each candidate UD test, check coverage of each assertion from the old test:

```
Old test: "should throw error for wrong okta credentials"
  Assertion 1 (AuthenticationError type)    → ✓ covered by test_fail_with_bad_credentials
  Assertion 2 (error message content)       → ✓ covered by test_fail_with_bad_credentials
  Assertion 3 (SQLSTATE 28000)              → ✗ NOT covered — UD test checks error type only
  Assertion 4 (no cached token)             → ✓ covered by test_cache_cleared_on_failure
  Assertion 5 (retry recovery)              → ✗ NOT covered — no UD test exercises this
```

#### 3c. Determine mapping completeness

- **Full coverage**: all old assertions are present in mapped UD test(s) → `status: mapped`
- **Partial coverage**: some assertions missing → `status: partial` with `gaps` list enumerating each uncovered assertion
- **Scope expansion**: UD test covers MORE than old test → valid mapping (`status: mapped`), note the expansion in `notes`
- **Scope reduction**: UD test covers LESS (even if name is similar) → `status: partial`, document what's missing in `gaps`

**Critical rule**: Two tests with similar names may have different assertion scopes. Never accept a mapping based on name similarity alone. Always verify assertion-by-assertion.

Accept `status: mapped` only when the mapped UD test(s) collectively cover ALL assertions from the old test. If coverage is partial, use `status: partial` with a structured `gaps` list — this makes gaps queryable and trackable, not buried in free-text.

#### 3d. Cross-check gaps against the BehaviorDifferences file (mandatory for every gap)

For every gap identified in Step 3b, **read the relevant BehaviorDifferences file** and check whether the divergence is already documented:

| Driver | File |
|---|---|
| ODBC | `odbc_tests/BehaviorDifferences.yaml` |
| Python | `python/BehaviorDifferences.yaml` |
| JDBC | `jdbc/BehaviorDifferences.yaml` |
| Node.js | `nodejs/BehaviorDifferences.yaml` |

**For each gap, one of three outcomes applies:**

1. **Gap is a known behavior difference** — cite it in the `gaps` entry:
   ```yaml
   gaps:
     - "Empty username raises DatabaseError instead of ProgrammingError (BehaviorDifference #33)"
   ```

2. **Gap looks like a behavior difference but is NOT in the file** — add a `notes` flag so it can be tracked:
   ```yaml
   gaps:
     - "SQLSTATE 28000 not propagated on auth failure — may be a deliberate UD simplification; not yet in BehaviorDifferences.yaml, consider adding"
   ```

3. **Gap is a missing UD test, not a behavioral difference** — document it plainly:
   ```yaml
   gaps:
     - "No UD test exercises retry-with-correct-credentials recovery path"
   ```

**Rules:**
- Read the BD file for every mapping session — do not rely on memory of its contents.
- If a gap belongs to outcome 2, propose the new BehaviorDifferences entry as part of your output (use the next available integer ID). Do not write it to the file without user confirmation.
- Do not conflate "gap" with "behavior difference": a gap means no UD test covers the assertion; a behavior difference means the UD intentionally behaves differently. Both can coexist on the same old-test entry.

---

### Step 4 — Update the YAML mapping file

Produce a YAML diff showing the proposed changes. Do not overwrite the file without user confirmation. Present the diff first:

```yaml
# Proposed addition to tests/oldTestsCoverage/odbc.yaml
tests:
  AuthenticationTests/AuthLatestTest/ExternalBrowserTest.cpp:
  - test_name: "should authenticate using external browser"   # NEW ENTRY
    ud_tests:
      - path: sf_core/tests/e2e/authentication/external_browser.rs - should_authenticate_using_external_browser
        verification: intent-only
    status: mapped
    notes: "Intent from SNOW-3548054 AC: browser launch opens login page"
    jira: "SNOW-3548054"
```

After user confirmation, apply the edit with the Edit tool.

---

### Step 5 — Coverage summary (screening gate)

Regenerate the HTML report to reflect the updated YAML state:

```bash
python3 tests/oldTestsCoverage/scripts/generate-report.py
```

If the script is absent, compute manually: count entries by status and report the ratio. Flag any old tests with `status: unmapped`.

---

### Step 6 — Reverse lookup

Given a UD test identifier, scan all mapping YAML files for entries that list it under `ud_tests`. Return the full set of old-driver tests that the UD test covers, grouped by driver.

## Output format

Every response from this skill includes four blocks in order:

**1. Scope confirmation** (1–3 lines)
```
Driver: odbc  |  Mode: add-mapping  |  Scope: TC_AUTH_001_BasicLogin
```

**2. Intent summary** (for add-mapping / gap modes)
One sentence per old test analyzed, citing source (Jira ticket or commit SHA).

**3. Proposed YAML diff**
Fenced YAML block showing only the changed/added lines, with `# NEW` or `# CHANGED` comments on modified keys.

**4. Coverage gap report** (always included)
```
ODBC coverage summary (after proposed changes):
  mapped:          184 / 432  (42.6%)
  partial:          30 / 432  ( 6.9%)
  verified:         38 / 432  ( 8.8%)
  not-applicable:   12 / 432  ( 2.8%)
  unmapped:        168 / 432  (38.9%)

Partial entries with gaps: 30 (total gaps: 47)
Orphaned UD tests (no old-test mapping): 7
  - tests::auth::test_token_refresh_race
  ... (run with --full for complete list)
```

## Quality rules

**Pass criteria:**
- Every new mapping entry has all required fields: `test_name`, `ud_tests` (non-empty list), `status`.
- YAML diffs are shown before file edits; the developer confirms before the Edit tool runs.
- Intent statement cites a Jira ticket or commit SHA, not inference alone, wherever possible.
- **Assertion-level analysis is mandatory**: every mapping must be backed by reading the full test body of BOTH the old test and the candidate UD test. Name similarity is never sufficient.
- **Gaps must be structured**: if any assertion from the old test is not covered, use `status: partial` with a `gaps` list (not free-text notes). Each gap entry describes one uncovered assertion.
- **`gaps` field is required when `status: partial`** and forbidden when `status: mapped`.
- **Search checklist must be complete**: all 11 rows must show "YES" before proposing a mapping.
- **BehaviorDifferences cross-check is mandatory**: for every `status: partial` entry, the relevant `BehaviorDifferences.yaml` must have been read. Each gap must be classified as: known BD (cite `#N`), candidate for new BD entry (flag it + propose the entry), or missing UD test.

**Fail / do not do:**
- Do not accept a mapping based on test name similarity alone — always read both test bodies and compare assertions.
- Do not set `status: mapped` without confirming ALL assertions from the old test are covered.
- Do not silently merge YAML — always diff first.
- Do not treat "similar scope" as "same scope" — if the old test checks 5 things and the UD test checks 3, that's a coverage gap even if the test names match.
- **Do not search only `tests/definitions/`** — feature files describe scenarios in Gherkin but NOT the actual assertions. You MUST search `sf_core/tests/`, `odbc_tests/tests/`, `python/tests/` for the implementation code that contains the real assertions.
- **Do not skip unit tests** — unit tests across all languages may directly cover old test assertions and are valid mappings: `sf_core/src/**/` (Rust inline `#[cfg(test)]`), `python/tests/unit/` (Python), `odbc_tests/tests/odbc-api/` + `odbc_tests/tests/basic_tests/` + `odbc_tests/tests/bindings_tests/` (C++), `nodejs/tests/unit/` (TypeScript), `jdbc/src/test/java/` (Java).
- **Do not declare row 3 "no match" without showing discovery evidence.** Bad output: `sf_core/src/ | YES | no match` — with no grep output shown. A "no match" is only credible when the response includes which keywords were searched, which files were found, and that none contained relevant `#[test]` functions. Example of correct "no match" output: "Searched `sf_core/src/` for keywords ['password', 'authenticator', 'connection']: found 6 files, 3 had `#[test]` functions, none matched the DSN-timeout behavior being mapped."
- **Do not skip the search checklist** — if any row is missing from your output, the mapping is invalid.

**Counterexample (bad — name-based mapping without body analysis):**
```yaml
- old_test: "TC_AUTH_001_BasicLogin"
  ud_tests:
    - path: "tests::auth::test_basic_login"   # mapped by name only, no code read
  status: mapped
  # Missing: old test also checks SQLSTATE code and retry behavior
```

**Correct (assertion-verified mapping with structured gaps):**
```yaml
- test_name: "should throw error for wrong okta credentials"
  ud_tests:
    - path: sf_core/tests/e2e/authentication/native_okta.rs - vpn_should_fail_native_okta_authentication_with_wrong_credentials
    - path: sf_core/tests/integration/authentication/native_okta.rs - should_fail_with_bad_credentials_when_okta_returns_401
  status: partial
  gaps:
    - "No UD test verifies SQLSTATE 28000 error code on auth failure"
    - "Recovery path (retry with correct credentials succeeds) not exercised"
  notes: "Assertions 1-2 covered (error type, message). 2 gaps need new UD tests."
  jira: "SNOW-3548054"
```

## Gotchas

- **ODBC first.** JDBC (1640 tests) and Python (1956 tests) are not yet piloted. If the user asks to map all JDBC tests at once, redirect: analyze a sample of 10–20 first to validate the workflow before bulk processing.
- **Jira MCP rate limits.** If fetching many tickets in sequence, batch reads and note any fetch failures rather than silently dropping context.
- **Many-to-many is expected.** A UD test appearing in 15 old-test entries is normal (a broad auth test may cover many old TC_ variants). Flag as a note, not an error.
- **Missing `test_list` sync.** If a test name appears in `mappings` but not in `test_list`, the YAML is inconsistent — report it and offer to add the entry to `test_list`.

## Out of scope

- **Writing new UD tests.** This skill maps existing tests; authoring new tests is out of scope. If a coverage gap has no candidate UD test, flag it as `unmapped` — a separate task should create the test.
- **Running CI pipelines end-to-end.** This skill produces YAML diffs and analysis; it does not trigger CI or submit PRs. For CI triggering, use the `merge-pr` skill.
- **Translating or transpiling old tests.** Old test code is analyzed for intent only; the skill does not port test logic into UD syntax.
- **Python or JDBC bulk mapping.** Until ODBC pilot is complete and the workflow is validated, redirect bulk JDBC/Python requests to start with a 10–20 test sample.

## Examples

**Example 1 — Add a single mapping**

User: `"Map old ODBC test TC_NET_042_ConnectionTimeout to a UD test"`

Skill output:
```
Driver: odbc  |  Mode: add-mapping  |  Scope: TC_NET_042_ConnectionTimeout

Intent (from SNOW-2901234 AC):
  Verifies that a login attempt times out and returns a client-visible
  error when the server does not respond within the configured timeout window.

Proposed YAML diff:
  # tests/oldTestsCoverage/odbc.yaml
  mappings:
    - old_test: "TC_NET_042_ConnectionTimeout"   # NEW
      ud_tests:
        - "tests::network::test_login_timeout_returns_error"
      status: mapped
      verification: intent-only
      notes: "SNOW-2901234: timeout behavior; mutation run pending"
      jira: "SNOW-2901234"

Confirm edit? (y/n)
```

---

**Example 2 — Coverage gap report**

User: `"Show me coverage gaps for ODBC"`

Skill output:
```
Driver: odbc  |  Mode: coverage-gap  |  Scope: all

ODBC coverage summary:
  mapped:          198 / 432  (45.8%)
  verified:         22 / 432  ( 5.1%)
  not-applicable:   10 / 432  ( 2.3%)
  unmapped:        202 / 432  (46.8%)

Top unmapped clusters by test prefix:
  TC_AUTH_*   48 unmapped
  TC_NET_*    37 unmapped
  TC_STMT_*   61 unmapped

Suggest starting with TC_AUTH_* (Jira tickets are well-tagged).
```

---

**Example 3 — Reverse lookup**

User: `"Which old ODBC tests does tests::auth::test_basic_login cover?"`

Skill output:
```
Driver: odbc  |  Mode: reverse-lookup  |  UD test: tests::auth::test_basic_login

Old tests mapping to this UD test:
  TC_AUTH_001_BasicLogin        (status: mapped,    verification: intent-only)
  TC_AUTH_003_CaseInsensitive   (status: verified,  verification: mutation-proven)
  TC_AUTH_017_EmptyPassword     (status: mapped,    verification: intent-only)

3 old tests covered. 2 additional old tests reference auth but map to
different UD tests (see TC_AUTH_005, TC_AUTH_009).
```
