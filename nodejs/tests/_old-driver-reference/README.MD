# Old Driver Test Reference

This folder is a snapshot of tests from the original
[snowflake-connector-nodejs](https://github.com/snowflakedb/snowflake-connector-nodejs) driver
(commit `415160294a48087b18b0a70bb09534078fe4afac`).

These tests are kept as a reference for migration — they are **not** run as-is.

## Why migrate?

The original test suite has several problems:

- Uses **Mocha** — we use **Vitest**.
- Written in **JavaScript** — we use **TypeScript**.
- Many tests mock driver internals instead of testing real end-to-end behavior.
- Test classification is inconsistent: some "unit" tests are really E2E, and most "integration" tests should be E2E.
- Most unit tests cover logic that should be (or already is) covered by `sf_core`.

## Migration plan

### Phase 1 — E2E test migration

1. Use an AI migration rules file to guide consistent, repeatable migration decisions.
2. Triage each test file with AI to determine if it should become an E2E test.
3. Migrate identified tests into a new Vitest-based E2E suite that can run against **both** the old driver and the new driver.
4. Remove each migrated test file from this folder.

### Phase 2 — Unit / integration test review

1. Review remaining unit and integration tests.
2. Use AI to check whether the behavior they cover already exists in `sf_core` tests.
   - If covered — drop the test.
   - If not covered — migrate the test or its underlying behavior into `sf_core`.
3. Remove each resolved test file from this folder.

### Done

Once every test has been migrated or dropped, **delete this folder entirely**.
