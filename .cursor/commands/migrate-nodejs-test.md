---
description: Migrate a single Node.js integration test file from `nodejs/tests/_old-driver-reference/integration/` into the new Vitest E2E suite at `nodejs/tests/e2e/`.
---

# Migrate Node.js Test

Migrate the file passed via `@filename` (expected to be a test file under `nodejs/tests/_old-driver-reference/integration/`) into `nodejs/tests/e2e/`.

Usage: `/migrate-nodejs-test @nodejs/tests/_old-driver-reference/integration/<file>.js`

## Workflow

Copy this checklist and track progress:

```
Migration Progress:
- [ ] 1. Read the source file passed via @filename
- [ ] 2. Triage tests (migrate / flag-for-sf_core / drop)
- [ ] 3. Propose new file name and confirm with user
- [ ] 4. Write the new `nodejs/tests/e2e/<name>.test.ts`
- [ ] 5. Verify it passes against the old driver
- [ ] 6. Remove migrated tests from the source file
- [ ] 7. If source file ends up with zero `it()` blocks, delete it
- [ ] 8. Summarise what was migrated, flagged, and dropped
- [ ] 9. Self-review this command file and update it if needed
```

### Step 1: Read the source file

Read the full file the user passed via `@filename`.

### Step 2: Triage tests

**Goal of the E2E suite:** verify each public method of the driver returns the expected result on
success and the expected error on failure — i.e. that the public API surface is wired up correctly
end-to-end. Edge cases, branching logic, parameter permutations, protocol/serialization details,
and internal state machines are **not** the E2E suite's responsibility — they belong in `sf_core`
(Rust), where the logic actually lives.

For each `it()` in the file, classify it as one of:

- **Migrate**: covers one positive and (optionally) one negative path of a public driver method — e.g. "a query can be cancelled", "connecting with bad credentials fails".
- **Flag for `sf_core`**: digs into edge cases, parameter matrices, or internals (reaches into `../../lib/...`, uses `wiremock` / `sinon` / `rewiremock` / `mock-require` to assert on internal behaviour). **Stop and notify the user** before migrating these — that coverage should be added to `sf_core`, not duplicated as slow Node.js E2E tests against a live account.
- **Drop**: already covered by something in `tests/e2e/`, or pure unit logic with no public-API surface.

When in doubt, migrate only the happy-path + one failure-mode case for each public method, and flag the rest for `sf_core` coverage.

### Step 3: Propose new file name

Test file: kebab-case `<topic>.test.ts` in `nodejs/tests/e2e/`. Propose the name to the user **before** writing the file and wait for confirmation (or a counter-suggestion) unless the mapping is unambiguous.

Examples:

| Source | Proposed |
|---|---|
| `testMultiStatement.js` | `multi-statement.test.ts` |
| `testConcurrent.js` | `concurrent-execution.test.ts` |
| `testExecuteAsync.js` | `async-execution.test.ts` |
| `testRowMode.js` | `row-mode.test.ts` |
| `testUpdatedRows.js` | `updated-rows.test.ts` |
| `testConnectionPoolExecute.js` | `connection-pool.test.ts` |
| `testPutSmallFiles.js` | `put-small-files.test.ts` |

### Step 4: Write the new test file

Apply these rules — they are the migration spec, follow them literally.

#### Framework

- Replace Mocha `describe` / `it` / `before` / `after` with Vitest `describe` / `it` / `beforeAll` / `afterAll` (imported from `vitest`).

#### Types

- Import types (`Connection`, `ConnectionOptions`, `RowStatement`, etc.) directly from `snowflake-sdk`.
  > Temporary: once the new universal driver SDK exposes its own type surface, types should be imported from there instead.
- Multi-statement helpers `hasNext()` and `NextResult()` are declared on `FileAndStageBindStatement`
  (which extends `RowStatement`) — not on `RowStatement` itself. When iterating sub-results, narrow
  the statement with `as FileAndStageBindStatement` rather than reaching for `as any`.
- When you have to cast around an incomplete `snowflake-sdk` type (e.g. the multi-statement helpers
  above, or any other gap in the upstream `.d.ts`), leave a short `// TODO:` comment at the cast
  site noting it's a missing-SDK-types gap, so it's easy to find and remove once the types catch up.

#### Connection lifecycle

- Replace `testUtil.createConnection(overrides?)` with `createConnection(overrides?)` from `tests/e2e/utils`.
  Default connection parameters (`SNOWFLAKE_TEST_ACCOUNT`, `SNOWFLAKE_TEST_USER`, `SNOWFLAKE_TEST_PASSWORD`,
  `SNOWFLAKE_TEST_WAREHOUSE`, `SNOWFLAKE_TEST_DATABASE`, `SNOWFLAKE_TEST_SCHEMA`, `SNOWFLAKE_TEST_ROLE`)
  **should already be built in** — pass only overrides. If a parameter is missing, add it to `createConnection`
  in `tests/e2e/utils/index.ts` rather than wiring it up in the test. Resolution order is defined by
  `tests/e2e/utils/getTestParameter.ts`: `parameters.json` (`testconnection` section, path from `PARAMETER_PATH`
  or repo root) first, then `process.env` as fallback.
- Replace `testUtil.connectAsync(conn)` with `connectAsync(conn)` from `tests/e2e/utils`.
- Replace `testUtil.destroyConnectionAsync(conn)` with `destroyAsync(conn)` from `tests/e2e/utils`.

#### Logger

- Drop all logger configuration (`snowflake.configure({ logLevel: ... })`, `Logger()` calls, etc.).
  The test harness does not configure logging.
- Drop log-only sub-steps that exist purely to print state (e.g. an `async.series` step that runs
  `select current_version()` just to log driver/server versions, or `Logger.getInstance().info(row)`
  inside a stream handler). They carry no assertion and are noise in the migrated test.

#### Callbacks to promises

- Convert `done()` callback patterns to `async` / `await`.
- For setup / teardown SQL (e.g. `alter session set ...` in `beforeAll`), prefer the
  `executeAsync(connection, sqlText, options?)` helper from `tests/e2e/utils` over an inline
  `new Promise` wrapper around `connection.execute({ ..., complete })`.
- Reserve the inline `new Promise` pattern for callback APIs the helpers don't cover — e.g.
  `statement.cancel(cb)`, or mid-stream interactions where you need access to the live `stmt`
  inside `streamRows()` (`hasNext()` / `NextResult()` walking).
- Use smaller timeouts where the exact delay is not semantically important.

#### Assertions

- Replace `assert.ok(!err)` / `testUtil.checkError(err)` with Vitest `expect()`.

#### Naming inside the file

- `describe` block: human-readable title case (e.g. `"Query Cancellation"`).

### Step 5: Verify

Run the migrated test against the old driver:

```bash
cd nodejs && npm run test:e2e-old-driver -- <new-file>.test.ts
```

The test must pass against the old driver. If it fails, fix it before proceeding.

### Step 6: Remove migrated tests from the source file

Delete **only** the `it()` blocks that were successfully migrated. Keep any `it()` blocks that were flagged for `sf_core` — they stay in the source file as a record until they are explicitly moved into `sf_core` or dropped.

If every `it()` inside a `describe` was migrated, remove the entire `describe`. Also remove any now-unused imports, `before` / `after` hooks, and helper variables. Do not leave dead code in the source file.

### Step 7: Delete the source file if empty

If after Step 6 the file contains zero `it()` blocks, delete it entirely.

### Step 8: Summary

End with a short summary covering:

- New file path
- Which tests were migrated (one line each)
- Which tests were flagged for `sf_core` (one-line reason each)
- Which tests were dropped (one-line reason each)
- Whether the source file was deleted, and if not, what remains in it

### Step 9: Self-review this command file

Before finishing, review **this very file** (`.cursor/commands/migrate-nodejs-test.md`) against what
actually happened during the migration just completed, and update it if anything is missing, wrong,
or unclear. The goal is for each migration to leave this file a little better than it found it.

Walk through these prompts and act on any "yes" answer by editing this file:

- **Hidden rule applied?** Did you make a judgement call that isn't written down here (e.g. how to
  handle a specific assertion shape, a Vitest API, a flaky pattern, a missing helper)? → Add the rule.
- **Stale or wrong instruction?** Did any instruction in this file mislead you or have to be ignored
  to get the migration through? → Fix the instruction.
- **New helper added to `tests/e2e/utils/`?** → Mention it in the "Reference" section so the next
  migration reuses it instead of re-adding it.
- **Naming example clarified?** Did you pick a file name that doesn't match any row in the Step 3
  table, and the mapping isn't obvious from the existing rows? → Add the new `<source> → <proposed>`
  row.
- **New "flag for `sf_core`" indicator?** Did you encounter a new internal-API marker (a `lib/...`
  import, a mocking library, a test-only env var) that signals "this isn't an E2E candidate"? → Add
  it to the bullet list in Step 2.
- **Step out of order?** Did the workflow above only work because you reordered or skipped a step? →
  Reorder the checklist and the section headings to match what actually works.

Keep edits surgical: only change what the latest migration proved needs changing. If nothing needs
changing, say so explicitly in the summary ("Self-review: no changes needed").

This step is **not optional** — skipping it means the next run repeats the same mistake.

## Reference: existing migrated tests

For style examples, see:
- `nodejs/tests/e2e/query-cancellation.test.ts`
- `nodejs/tests/e2e/connection-serialization.test.ts`
- `nodejs/tests/e2e/multi-statement.test.ts`
- `nodejs/tests/e2e/utils/index.ts` (helpers — `createConnection`, `connectAsync`, `destroyAsync`, `executeAsync`, `sleepAsync`, `getSnowflakeSDK`)
