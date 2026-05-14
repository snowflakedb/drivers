# Migration Rules

Apply these rules when migrating any test file from this folder into `tests/e2e/`.

## Triage before migrating

**Goal of the E2E suite:** verify each public method of the driver returns the expected result on
success and the expected error on failure — i.e. that the public API surface is wired up correctly
end-to-end. Edge cases, branching logic, parameter permutations, protocol/serialization details,
and internal state machines are **not** the E2E suite's responsibility — they belong in `sf_core`
(Rust), where the logic actually lives.

Before mechanically converting a file, evaluate **what** it actually tests:

- A good E2E candidate covers one positive and (optionally) one negative path of a public driver
  method — e.g. "a query can be cancelled", "connecting with bad credentials fails".
- If the file goes beyond that and digs into edge cases or internals, **stop and notify the user**:
  that coverage should be added to `sf_core`, not duplicated as slow Node.js E2E tests against a
  live account.
- When in doubt, migrate only the happy-path + one failure-mode case for each public method, and
  flag the rest for `sf_core` coverage.

## Framework

- Replace Mocha `describe`/`it`/`before`/`after` with Vitest `describe`/`it`/`beforeAll`/`afterAll` (imported from `vitest`).

## Types

- Import types (`Connection`, `ConnectionOptions`, `RowStatement`, etc.) directly from `snowflake-sdk`.
  > Temporary: once the new universal driver SDK exposes its own type surface, types should be imported from there instead.

## Connection lifecycle

- Replace `testUtil.createConnection(overrides?)` with `createConnection(overrides?)` from `tests/e2e/utils`.
  Default connection parameters (`SNOWFLAKE_TEST_ACCOUNT`, `SNOWFLAKE_TEST_USER`, `SNOWFLAKE_TEST_PASSWORD`,
  `SNOWFLAKE_TEST_WAREHOUSE`, `SNOWFLAKE_TEST_DATABASE`, `SNOWFLAKE_TEST_SCHEMA`, `SNOWFLAKE_TEST_ROLE`)
  **should already be built in** — pass only overrides. If a parameter is missing, add it to `createConnection`
  in `tests/e2e/utils/index.ts` rather than wiring it up in the test. Resolution order is defined by
  `tests/e2e/utils/getTestParameter.ts`: `parameters.json` (`testconnection` section, path from `PARAMETER_PATH`
  or repo root) first, then `process.env` as fallback.
- Replace `testUtil.connectAsync(conn)` with `connectAsync(conn)` from `tests/e2e/utils`.
- Replace `testUtil.destroyConnectionAsync(conn)` with `destroyAsync(conn)` from `tests/e2e/utils`.

## Logger

- Drop all logger configuration (`snowflake.configure({ logLevel: ... })`, `Logger()` calls, etc.).
  The test harness does not configure logging.

## Callbacks to promises

- Convert `done()` callback patterns to `async`/`await`.
- Wrap remaining callback-based SDK methods (e.g. `statement.cancel(cb)`) in a `new Promise` inline.
- Use smaller timeouts where the exact delay is not semantically important.

## Assertions

- Replace `assert.ok(!err)` / `testUtil.checkError(err)` with Vitest `expect()`.

## Naming and location

- Test file: kebab-case `<topic>.test.ts` in `tests/e2e/`.
- `describe` block: human-readable title case (e.g. `"Query Cancelation"`).

## After migration

- Verify the migrated test passes against the old driver (`npm run test:e2e-old-driver`).
- Delete the original test file from this folder.
