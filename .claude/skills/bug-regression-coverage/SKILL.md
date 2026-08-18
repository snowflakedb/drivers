---
name: bug-regression-coverage
description: >
  Checks UD regression coverage for a specific Jira bug ticket identified by
  a SNOW-XXXXXX ticket number. Use when the user says: 'check bug
  regression', 'regression coverage for SNOW-', 'bug coverage check', 'does
  UD cover this bug', 'verify bug in UD', '/bug-regression-coverage'. Always
  requires a SNOW-XXXXXX ticket key — does NOT handle test file mapping, old
  test names, YAML coverage files, or coverage gap reports across driver test
  suites (use test-coverage-mapper for those).
---

## Opening context

This skill takes a Jira bug ticket (SNOW-XXXXXX) and answers two distinct
questions via a two-track approach:

**Track 1 — Regression verification (always run, no confirmation needed):**
Write a *temporary regression test* — a proper test file placed in the UD test
directory, following the same conventions as other tests in that folder, and
run via the same wrapper skill (`run-python-ud-tests`, `run-odbc-ud-tests`,
etc.). This proves whether the UD has the regression *right now* and is the
primary output of this skill.

**Track 2 — Formal coverage audit (run after Track 1):**
Search the UD's permanent test suite for assertions that cover the same
scenario. Report gaps and, after user confirmation, promote the temporary
regression test (or write new tests) as permanent UD tests that guard against
the regression in CI forever.

The two tracks are complementary — Track 1 gives an immediate answer; Track 2
ensures the answer stays true over time.

The source YAML files listing known fixed bugs are:
- `tests/bugs_analysis/python.yaml` (Python driver)
- `tests/bugs_analysis/odbc.yaml` (ODBC driver)
- `tests/bugs_analysis/jdbc.yaml` (JDBC driver, if present)
- `tests/bugs_analysis/nodejs.yaml` (Node.js driver, if present)

---

## Workflow

### Step 0 — Pre-flight: credentials and compiled core

Before running any UD tests, check two things in parallel:

**Credentials** (`parameters.json`):
```bash
ls parameters.json 2>/dev/null && echo "present" || echo "missing"
```

**Compiled Rust core** (Python only — macOS example):
```bash
ls python/src/snowflake/connector/_core/*.dylib 2>/dev/null && echo "present" || echo "missing"
```

For full setup instructions, build commands, and env vars, invoke the
appropriate wrapper runbook skill:

- **Python** → `run-python-ud-tests` skill
- **ODBC** → `run-odbc-ud-tests` skill
- **JDBC** → `run-jdbc-ud-tests` skill
- **Node.js** → `run-nodejs-ud-tests` skill

**Important:** Gap analysis (Steps 1–7) does **not** require credentials or
a compiled core — proceed without them. Only Step 8b (running UD tests)
needs them. If either is missing, ask the user:
> "Credentials / compiled core missing. Shall I continue with gap analysis
> only, or pause while you set up the environment?"

### Step 1 — Identify the ticket

If the user provided a ticket key (SNOW-XXXXXX), use it directly.

If no key was provided, ask:
> "No ticket key provided. Please give me a SNOW-XXXXXX key, or should I pick
> one from the bugs_analysis YAML files?"

Use `jira_get_issue` to fetch: summary, description, acceptance criteria,
affected component, fix version, and any linked PRs or commits.

If the Jira MCP is unavailable, ask the user to paste the ticket description
manually.

### Step 2 — Determine the affected driver

From the ticket component, summary, and description decide which driver is
affected. Quick reference: Python → `snowflakedb/snowflake-connector-python`
(`~/snowflake-connector-python`); ODBC → `snowflakedb/snowflake-odbc`
(`~/snowflake-odbc`); JDBC → `snowflakedb/snowflake-jdbc` (`~/snowflake-jdbc`);
Node.js → `snowflakedb/snowflake-connector-nodejs`
(`~/snowflake-connector-nodejs`). Full repo table and GitHub MCP fallback
instructions:

@.claude/rules/driver-repos.md

If ambiguous, ask the user which driver to target before proceeding.

### Step 3 — Locate or write a regression test in the original driver repo

This step is for **understanding the bug scenario only** — the test runs
against the original driver (not the UD) to confirm the fix exists there.

Search the original repo for an existing test. Grep for: the ticket key,
relevant function names, error strings, or API parameters.

- **Test exists** → confirm its path and read it in full.
- **No test exists** → write a minimal focused test that directly exercises the
  bug scenario. State the proposed path and ask the user to confirm before
  writing. Mark it with: `# Regression: SNOW-XXXXXX`

### Step 4 — Run the original driver test (fix verification only)

Run the test in the original driver repo to confirm the fix is present.
This is **not** a UD test run — UD testing happens in Steps 8b and on.

**Python driver** (`~/snowflake-connector-python`):
```bash
# Credentials go in test/parameters.py:
# CONNECTION_PARAMETERS = {'account': '...', 'user': '...', 'password': '...', ...}

cd ~/snowflake-connector-python

# Activate the tox environment first (preferred):
. .tox/py39/bin/activate
pytest -v <path/to/test.py>::<test_name>

# Or run directly with pytest:
python -m pytest <path/to/test.py>::<test_name> -v
```

**ODBC driver** (`~/snowflake-odbc`):
```bash
# 1. Build (if not already built):
cd ~/snowflake-odbc/Installer && ./gen_unix_installer.sh

# 2. Configure ODBC environment:
cd ~/snowflake-odbc
python ConfigureTemplate/gen_conf.py   # generates conf/ directory
export ODBCSYSINI=~/snowflake-odbc/conf
export SIMBAINI=~/snowflake-odbc/conf/unixodbc.snowflake.ini

# 3. Run the test binary from its own directory (required):
# cppunit test:
cd cmake_build/Tests/EndToEndTests/<TestName> && ./<TestName>Runner
# catch test:
cd cmake_build/Tests/EndToEndTests/<TestName> && ./<TestName>
```

**JDBC driver** (`~/snowflake-jdbc`):
```bash
# Credentials: set in environment or Maven settings
export SNOWFLAKE_TEST_ACCOUNT=<account>
export SNOWFLAKE_TEST_USER=<user>
export SNOWFLAKE_TEST_PASSWORD=<password>

cd ~/snowflake-jdbc

# Run a specific test class:
mvn test -Dtest=<TestClassName> -pl . -am

# Run a specific test method:
mvn test -Dtest=<TestClassName>#<methodName> -pl . -am
```

**Node.js driver** (`~/snowflake-connector-nodejs`):
```bash
# Credentials: set in environment variables
export SNOWFLAKE_TEST_ACCOUNT=<account>
export SNOWFLAKE_TEST_USER=<user>
export SNOWFLAKE_TEST_PASSWORD=<password>
export SNOWFLAKE_TEST_DATABASE=<database>
export SNOWFLAKE_TEST_SCHEMA=<schema>
export SNOWFLAKE_TEST_WAREHOUSE=<warehouse>

cd ~/snowflake-connector-nodejs

# Run a specific test file:
npm test -- <path/to/test_file.js>
# Or with mocha directly:
npx mocha <path/to/test_file.js>
```

Interpret the result:
- **PASS** → the fix is confirmed present; continue to Step 5.
- **FAIL** → report the failure and ask whether to continue to gap analysis.
- **Missing credentials** → ask the user:
  > "Original driver test needs credentials.
  > Python: create `test/parameters.py` with CONNECTION_PARAMETERS dict.
  > ODBC: run `python ConfigureTemplate/gen_conf.py` and fill in account/user/password.
  > JDBC/Node.js: set SNOWFLAKE_TEST_ACCOUNT, SNOWFLAKE_TEST_USER, SNOWFLAKE_TEST_PASSWORD env vars.
  > Or confirm I should proceed to gap analysis without a live run."

### Step 5 — Extract assertions from the original test

Read the regression test and enumerate every assertion:
- **Python**: `assert` statements, `pytest.raises` blocks, `assertEqual` /
  `assertIn` / `assertRaises` calls.
- **ODBC**: `ASSERT_*` / `REQUIRE` / `CHECK` macros (Catch2), expected return
  codes, result-set checks, expected error code values.
- **JDBC**: `assertEquals` / `assertNotNull` / `assertThrows` / `assertTrue`
  (JUnit), expected `SQLException` codes or messages.
- **Node.js**: `expect(...)` assertions (Vitest/Chai), `toThrow` / `toBe` /
  `toEqual` / `rejects` matchers.

For each assertion, note the behavior it validates (one sentence).

---

## Track 1 — Regression verification (always, no confirmation needed)

### Step 6 — Write a temporary regression test and run it against the UD

**Do this without asking for user confirmation.** This is the primary answer
to "does the UD have this regression?".

Write a *temporary regression test* — a proper test file in the UD test
directory that exercises every assertion from Step 5. Naming convention:

| driver | File location | Name pattern |
|---|---|---|
| Python | `python/tests/unit/` or `python/tests/integ/` | `test_regression_snow_XXXXXX.py` |
| ODBC | `odbc_tests/tests/e2e/` | `regression_snow_XXXXXX.cpp` |
| JDBC | `jdbc/src/test/java/…/` | `RegressionSnowXXXXXXTest.java` |
| Node.js | `nodejs/tests/e2e/` | `regression-snow-XXXXXX.test.ts` |

The test must:
- Cover every assertion from Step 5.
- Follow the naming and fixture conventions of neighbouring tests in the same
  directory (read 2–3 existing files first).
- Be marked at the top: `# Temporary regression test: SNOW-XXXXXX` (or
  language-equivalent comment).
- Be a minimal, focused test — not a full feature test suite.

Run it immediately using the **same wrapper skill** as all other UD tests:
- **Python** → `run-python-ud-tests` skill (`hatch run dev:unit -k test_regression_snow_XXXXXX`)
- **ODBC** → `run-odbc-ud-tests` skill (`./odbc_tests/run.sh -R "snow_XXXXXX"`)
- **JDBC** → `run-jdbc-ud-tests` skill (`./gradlew test --tests "RegressionSnowXXXXXXTest"`)
- **Node.js** → `run-nodejs-ud-tests` skill (`cd nodejs && npm run build:core && npm run test:e2e -- regression-snow-XXXXXX`)

Interpret the result:
- **PASS** → UD has no regression for this ticket. Continue to Track 2.
- **FAIL** → UD has the regression. Report it clearly and stop — ask the user
  whether to investigate the UD implementation or continue to Track 2 anyway.
- **Cannot run** (missing credentials / core not built) → report the blocker,
  note which assertions could not be verified, and continue to Track 2. Mark
  the temporary test result as `UNVERIFIED` in the output.

---

## Track 2 — Formal coverage audit

### Step 7 — Search the UD test suite for coverage

Search for existing *permanent* UD tests that cover each assertion from Step 5
(distinct from the temporary regression test written in Step 6):
- **Python driver bugs** → search `python/tests/` in this repo.
- **ODBC bugs** → search `odbc_tests/tests/` in this repo.
- **JDBC bugs** → search `jdbc/src/test/` in this repo.
- **Node.js bugs** → search `nodejs/tests/` in this repo.

For each assertion, grep for: the function or API under test, the error string
or code, the behavioral pattern. Mark each as **covered** or **gap**.

A partial hit (function name appears but the specific assertion is different)
is a **gap**, not covered.

### Step 8 — Report gaps and propose permanent UD tests

Produce the full gap report (see Output format). For each gap, propose a
permanent UD test. If a temporary regression test already covers the same
assertion (Step 6), note that it can be promoted rather than rewritten.
Do **not** write anything yet.

Ask:
> "Found N coverage gap(s). Shall I write/promote these as permanent UD tests?
> (y/n or select individual items)"

Also add a **General guidelines** section summarising what category of UD
coverage appears to be systematically missing.

### Step 9 — Write permanent UD tests (after user confirmation only)

After explicit user approval:

**9a. Write or promote the tests.** If the temporary regression test from
Step 6 already covers a gap assertion, move/rename it to a permanent location
and remove the `Temporary regression test:` marker. Otherwise write new tests
from scratch. Mark permanent tests with:
```python
# Regression coverage: SNOW-XXXXXX
```

**9b. Run the permanent tests** using the same wrapper skill as Step 6 to
confirm they pass. If any fail, report the output and ask whether to fix the
test, fix the UD implementation, or skip.

### Step 10 — Update analysis_status in the bugs_analysis YAML

After both tracks are complete, update the `analysis_status` field for this
ticket in the appropriate YAML file:
- Python bugs: `tests/bugs_analysis/python.yaml`
- ODBC bugs: `tests/bugs_analysis/odbc.yaml`
- JDBC bugs: `tests/bugs_analysis/jdbc.yaml`
- Node.js bugs: `tests/bugs_analysis/nodejs.yaml`

Set the field to one of:
- `covered` — Track 1 passed AND every assertion has permanent UD coverage
- `gap` — Track 1 passed but one or more permanent coverage gaps remain
- `regression` — Track 1 failed; UD has the bug
- `not_implemented_in_ud` — the fix exists in the driver but the UD does
  not yet implement this feature; mark to revisit later
- `n/a` — ticket is no longer relevant to UD (obsolete, superseded, or
  applies only to the original driver's internal logic)

If the ticket key is not present in the YAML (not tracked), skip this step.

---

## Output format

```
## Bug Regression Coverage: SNOW-XXXXXX

**Summary:** <one-line ticket summary>
**driver:** Python driver | ODBC driver
**Original test:** <path> — PASS | FAIL | NOT FOUND (written at <path>)

### Assertions in original test
1. <assertion description>
2. <assertion description>
3. <assertion description>

---

## Track 1 — Regression verification
**Temporary regression test:** <path/to/test_regression_snow_XXXXXX.py>
**Run via:** `hatch run dev:unit -k test_regression_snow_XXXXXX` (or equivalent)
**Result:** ✅ PASS | ❌ FAIL | ⚠️ UNVERIFIED (reason: <missing credentials / core>)

1. <assertion description> — ✅ PASS | ❌ FAIL | ⚠️ UNVERIFIED
2. <assertion description> — ✅ PASS | ❌ FAIL | ⚠️ UNVERIFIED
3. <assertion description> — ✅ PASS | ❌ FAIL | ⚠️ UNVERIFIED

---

## Track 2 — Formal coverage audit
1. <assertion description> — ✅ Covered by <ud_test_file>:<line>
2. <assertion description> — ❌ Gap
3. <assertion description> — ❌ Gap

### Coverage gaps and proposed permanent UD tests
#### Gap 1: <assertion description>
- **Proposed file:** python/tests/test_<feature>.py
- **Test name:** test_<behavior>_snow_<ticket_number>
- **Action:** Promote temporary regression test | Write new test
- **Sketch:** assert that <condition> when <scenario>

### General guidelines
<One paragraph on what category of assertions is systematically missing
from the UD test suite based on this analysis.>

### Verdict
Track 1: X/Y assertions verified against UD (PASS | FAIL | UNVERIFIED).
Track 2: X of Y assertions have permanent coverage. Z new UD tests proposed.
<Awaiting confirmation to write/promote permanent tests. | Tests written at: ...>
```

---

## Quality rules

**Must pass:**
- A temporary regression test is always written and run (Track 1) — never
  skipped silently. If it cannot run, the reason must be reported and the
  result marked `UNVERIFIED`.
- The temporary regression test is run using the **same wrapper skill** as
  regular UD tests — never as a standalone script outside the test framework.
- Every assertion in the original test is accounted for in both tracks —
  no silent omissions.
- The original regression test was actually run (or the user was explicitly
  asked whether to skip the run).
- Proposed permanent test names describe the scenario, not just the ticket
  number (e.g. `test_fetchmany_respects_size_limit_snow_1234` not `test_bug_fix`).
- User confirmation was obtained before writing any **permanent** UD tests.
- General guidelines section is always present, even if it says "coverage
  appears complete for this category."

**Must not:**
- ❌ Skip Track 1 (temporary regression test) and jump straight to Track 2.
- ❌ Run the temporary regression test as a standalone Python script — use
  the wrapper skill so it runs in the real test environment.
- ❌ Report "no gaps" without actually searching the UD test directories.
- ❌ Write permanent UD tests without explicit user approval.
- ❌ Treat a partial grep hit as "covered" when the specific assertion differs.
- ❌ Give up when credentials are missing — mark as UNVERIFIED and continue.

---

## Gotchas

- **UD credentials and core build**: See Step 0 and the wrapper runbook
  skills (`run-python-ud-tests`, `run-odbc-ud-tests`, etc.). `parameters.json`
  is GPG-encrypted — ask the user to run `./scripts/decode_secrets.sh`.
  Original driver tests use different credential files: Python uses
  `test/parameters.py`; ODBC uses `ConfigureTemplate/gen_conf.py`. Keep
  the two separate when reporting what is missing.
- **Python UD requires compiled Rust core even for unit tests**: The driver
  imports `c_api.py` at package init time. If `hatch run dev:unit` fails with
  `RuntimeError: Couldn't load core driver dependency`, consult the
  `run-python-ud-tests` skill.
- **Temporary regression test placement**: Put it in `unit/` if it needs no
  live connection, `integ/` if it does. A test in the wrong category will be
  skipped or fail when credentials are absent.
- **Promoting vs rewriting**: If the temporary regression test already covers a
  gap assertion, prefer promoting (removing the `Temporary regression test:`
  marker and moving to the right permanent location) over writing a duplicate.
- **Sparse tickets**: Some bugs have no reproduction steps. Ask the user to
  describe the scenario before writing a test.
- **UD structure varies**: Before writing a new UD test, read 2–3 existing test
  files in the target directory to match fixtures and naming conventions.
- **Multiple drivers affected**: If the ticket mentions both Python and ODBC,
  ask which to analyze first.
- **No UD test directory for this driver yet**: Ask the user for the correct
  path rather than creating a new directory structure unilaterally.
- **Ticket in bugs_analysis YAML but no Jira data**: Fall back to the YAML
  summary field and ask the user to supplement if needed.

---

## Out of scope

- Does not fix bugs in the driver or UD — only verifies test coverage.
- Does not analyze performance regressions or flaky tests.
- Does not cover Rust sf_core directly — only the four language drivers (Python, ODBC, JDBC, Node.js).
- Does not push or commit written tests — leaves that to the user or the
  `commit` skill.
- Does not triage unrelated CI failures — flags them but stays on-task.

---

## Examples

**Example 1 — Track 1 passes, partial formal coverage**

User: `regression coverage for SNOW-1234567`

Fetches SNOW-1234567 (Python driver, cursor.fetchmany size bug). Finds
`tests/test_cursor.py::test_fetchmany_size` in `~/snowflake-connector-python`.
Runs it — PASS. Extracts 3 assertions.
**Track 1:** Writes `python/tests/unit/test_regression_snow_1234567.py`,
runs `hatch run dev:unit -k test_regression_snow_1234567` — all 3 PASS.
**Track 2:** Searches `python/tests/` — row count and column type covered,
no-truncation assertion missing. Reports 1 gap, notes the temporary regression
test can be promoted. Asks for confirmation before promoting.

**Example 2 — Track 1 unverified (missing credentials), all gaps**

User: `check if SNOW-9876543 is covered in UD`

Fetches ticket (ODBC, connection reset error). No existing test in
`~/snowflake-odbc`. Writes minimal test — PASS. Extracts 2 assertions.
**Track 1:** Writes `odbc_tests/tests/e2e/regression_snow_9876543.cpp`, attempts
`./odbc_tests/run.sh -R snow_9876543` — fails: `parameters.json` missing.
Reports both assertions as UNVERIFIED.
**Track 2:** Searches `odbc_tests/tests/` — both gaps. Proposes 2 permanent
tests. Awaits user confirmation.

**Example 3 — Track 1 reveals regression**

User: `analyze bug ticket SNOW-1112233`

Fetches ticket (Python driver, temp stage name collision). Extracts 2
assertions.
**Track 1:** Writes `python/tests/unit/test_regression_snow_1112233.py`, runs
`hatch run dev:unit -k test_regression_snow_1112233` — 1 assertion FAILS.
Reports: "UD has this regression — `generate_temp_name` uses `random` not
`secrets`." Stops and asks the user whether to investigate the UD
implementation or proceed to Track 2 anyway.
