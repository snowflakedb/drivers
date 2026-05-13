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

## How it works

Migrated tests live in `tests/e2e/` and run against **both** drivers via two Vitest projects:

| Script | Vitest project | Driver |
|---|---|---|
| `npm run test:e2e` | `e2e` | new SDK (`src/index.ts`) |
| `npm run test:e2e-old-driver` | `e2e-old-driver` | `snowflake-sdk` package |

The `e2e-old-driver` project sets `SNOWFLAKE_NODEJS_E2E_USE_OLD_DRIVER=1`; shared helpers in `tests/e2e/utils/` use this to swap the underlying SDK at runtime.

CI runs both via `.github/workflows/test-nodejs.yml`.

## Migration plan

### Phase 1 — E2E test migration

1. Triage each test file to determine if it should become an E2E test.
2. Migrate using the rules in [MIGRATION_RULES.md](MIGRATION_RULES.md).
3. Verify the migrated test passes against both drivers.
4. Delete the original test file from this folder.

### Phase 2 — Unit / integration test review

1. Review remaining unit and integration tests.
2. Check whether the behavior they cover already exists in `sf_core` tests.
   - If covered — drop the test.
   - If not covered — migrate the test or its underlying behavior into `sf_core`.
3. Remove each resolved test file from this folder.

### Done

Once every test has been migrated or dropped, **delete this folder entirely**.
