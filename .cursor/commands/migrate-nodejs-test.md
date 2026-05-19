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
- **Drop**: already covered by something in `tests/e2e/`, pure unit logic with no public-API surface, **or exercises a server property rather than a driver property** (see below).

When in doubt, migrate only the happy-path + one failure-mode case for each public method, and flag the rest for `sf_core` coverage.

**Driver property vs. server property.** Before migrating, ask: "If I swap the SQL in this test for
different SQL that goes through the same public driver method, does the test still assert something
new about the *driver*?" If the answer is no, it's testing the server, not the driver, and should be
**dropped**. Concrete examples from past migrations:

- ✅ Driver: "a query can be cancelled" (`statement.cancel`), "connection.serialize() returns a JSON
  string", "10 concurrent `execute()` calls on one connection all complete with correct rows" (the
  driver multiplexes), "10 concurrent `execute()` calls on 10 independent connections all complete"
  (independent connection state).
- ❌ Server: "10 concurrent `create table` statements succeed" (same driver code path as any other
  concurrent `execute()` — only the SQL differs), "joining two tables returns the right rows",
  anything that's really asserting the cluster does its job.

**Strip session-setting setup unless the assertion depends on it.** Legacy tests often start with
`alter session set <foo> = <bar>` in `before(...)`. Migrate the session setting only if removing it
would change what the assertion checks. Common cases to **drop**: `use_cached_result = false` for
tests that assert on row counts (the cache returns the same rows), timezone/format settings for
tests that don't read date/time output, etc. If you keep the setting, the test description or a
short comment should make it obvious *why* the setting matters.

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
- Prefer the public `ErrorCode` enum re-exported from `snowflake-sdk` over reaching into
  `lib/errors` for error-code constants. A legacy `require('../../lib/errors').codes.ERR_FOO` is
  *not* an "internal API" red flag for migration purposes — `import { ErrorCode } from 'snowflake-sdk'`
  exposes the same values.
- When you have to cast around an incomplete `snowflake-sdk` type (any gap in the upstream
  `.d.ts`), leave a short `// TODO:` comment at the cast site noting it's a missing-SDK-types gap,
  so it's easy to find and remove once the types catch up.
- Casts that just narrow a correctly-typed union are **not** SDK-types gaps and should **not** carry
  a TODO. The most common case: `executeAsync` returns `{ statement: RowStatement | FileAndStageBindStatement, ... }`,
  so when a test only needs the row-statement surface, `statement as RowStatement` is a plain
  narrowing cast — no comment.

Known SDK-types gaps so far (use these as the canonical examples of what counts as a "gap"):

- `FileAndStageBindStatement.hasNext()` / `NextResult()` — declared only on
  `FileAndStageBindStatement`; cast with `as FileAndStageBindStatement` for multi-statement iteration.
- `Connection.getQueryStatus()` — typed `Promise<string>` but the `QueryStatus` literal union is
  what `isStillRunning()` accepts. Cast with `as QueryStatus`.
- `Connection.isAnError()` — declared with no args in the public types but the runtime takes a
  status string. Cast with `as (s: string) => boolean` (or `as (s: QueryStatus) => boolean`).

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
- **Default to a shared connection in `beforeAll` / `afterAll`** (the same shape the legacy Mocha
  tests use, and what the rest of the e2e suite does). Connect/destroy is the slowest part of an
  e2e run; reusing one connection across the whole `describe` keeps the suite fast. Only switch to
  per-test connections when a test needs its own connection state (e.g. a test that destroys its
  connection mid-flight, asserts on connection-id uniqueness, or needs different connection
  options from the rest of the file). When you do go per-test, wrap the lifecycle in a
  `try { ... } finally { destroyAsync(conn) }` — see "Resource cleanup" below.

#### Logger

- Drop all logger configuration (`snowflake.configure({ logLevel: ... })`, `Logger()` calls, etc.).
  The test harness does not configure logging.
- Drop log-only sub-steps that exist purely to print state (e.g. an `async.series` step that runs
  `select current_version()` just to log driver/server versions, or `Logger.getInstance().info(row)`
  inside a stream handler). They carry no assertion and are noise in the migrated test.

#### Callbacks to promises

- Convert `done()` callback patterns to `async` / `await`.
- Anywhere you'd otherwise hand-roll `new Promise((resolve, reject) => connection.execute({ ...,
  complete: ... }))`, use the `executeAsync(connection, sqlText, options?)` helper from
  `tests/e2e/utils` instead. This applies to setup/teardown SQL (`alter session set ...` in
  `beforeAll`) *and* to test-body calls where you only need the resulting statement (e.g. to read
  `statement.getQueryId()` after an `asyncExec: true` dispatch).
- Reserve the inline `new Promise` pattern for callback APIs the helpers don't cover — e.g.
  `statement.cancel(cb)`, or mid-stream interactions where you need access to the live `stmt`
  inside `streamRows()` (`hasNext()` / `NextResult()` walking).
- If the same callback / streaming pattern is repeated more than once **within the same migrated
  file** (e.g. "stream all rows and return the count"), extract a small module-local helper at the
  top of the file rather than inlining the `new Promise` twice. Only promote a helper to
  `tests/e2e/utils/` once it's needed by a second test file.
- Use smaller timeouts where the exact delay is not semantically important.

#### Resource cleanup

- Any test that opens its own `Connection` (i.e. not using the shared `beforeAll` connection) or
  creates cluster-scoped resources (tables, stages, file formats, etc.) must wrap the cleanup in
  `try { ... } finally { ... }` so a failing assertion doesn't leak state. The shared
  `beforeAll` / `afterAll` connection is already cleaned up by the harness — no `try/finally`
  needed around individual `it()`s for that.
- For best-effort destroy of multiple connections in a `finally`, swallow individual errors so one
  bad destroy doesn't mask the real test failure:
  `await Promise.all(conns.map((c) => destroyAsync(c).catch(() => undefined)))`.
  For a single connection in a `finally`, let `destroyAsync` propagate — the swallowing pattern is
  only for the multi-connection case.

#### Assertions

- Replace `assert.ok(!err)` / `testUtil.checkError(err)` with Vitest `expect()`.
- For "this promise should reject with a particular shape", prefer
  `await expect(promise).rejects.toMatchObject({ code: ..., name: ... })` over the legacy
  `try { await promise; assert.fail(); } catch (err) { assert.strictEqual(err.code, ...); }`
  pattern. It's shorter and the failure message is much better.
- Drop tautological constant lookups when migrating status / enum assertions. The legacy form
  `assert.strictEqual(QueryStatus[status], QueryStatus.SUCCESS)` is just
  `expect(status).toBe('SUCCESS')` — both sides of the legacy check resolve to the same string,
  the indirection adds nothing. Apply the same rule whenever you see an assertion that looks up
  an enum on both sides of an equality.
- **Fan-out tests must use distinct values per worker.** When a test runs N concurrent calls
  through the same driver method and asserts on the per-call result, each call should use a
  *different* input that produces a *different* expected output, and the assertion should compare
  position-for-position (e.g. `expect(actuals).toEqual(expecteds)`). The lazy shape
  `expect(rowCounts).toEqual(Array(N).fill(SAME_VALUE))` cannot catch a bug where results from one
  in-flight statement bleed into another. Prefer hardcoded distinct constants per test (e.g.
  `const expectedRowCounts = [2837, 6104, 1592, 8471, 3963]`) over `Math.random()` — tests must be
  deterministic.

#### Test organisation

- When a single source file covers multiple methods of the same surface, nest one
  `describe('<methodName>()')` per method inside the outer file-level `describe`. This reads much
  better than a flat list of `it()`s prefixed with the method name.
- When several `it()`s inside the *same* nested `describe` need identical per-test setup, lift it
  into a `beforeEach` and store the result in a `let` declared at the same scope. Do **not** put a
  `beforeEach` on a `describe` whose tests don't all need the setup — every `it()` in that
  describe pays for it. If only some tests need the setup, either keep it inline in those tests or
  split the describe.

#### Naming inside the file

- Outer file-level `describe`: human-readable title case (e.g. `"Query Cancellation"`,
  `"Async Query Execution"`).
- Inner method-grouping `describe`s: bare method signature with parens
  (e.g. `"getQueryStatus()"`, `"getResultsFromQueryId()"`).

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
- `nodejs/tests/e2e/query-cancellation.test.ts` — minimal shared-connection shape, callback API
  wrapped with inline `new Promise` (`statement.cancel`).
- `nodejs/tests/e2e/connection-serialization.test.ts` — `it.skip` with TODO link for known driver
  bugs; using `getSnowflakeSDK()` directly.
- `nodejs/tests/e2e/multi-statement.test.ts` — multi-statement iteration with the
  `FileAndStageBindStatement` cast.
- `nodejs/tests/e2e/concurrent-execution.test.ts` — fan-out via `Promise.all`, distinct expected
  values per worker, multi-connection cleanup pattern.
- `nodejs/tests/e2e/query-execution-async.test.ts` — nested `describe`s grouped by SDK method,
  `executeAsync` reused inside the test body for queryId setup, `beforeEach` to lift duplicated
  setup, `expect(...).rejects.toMatchObject({ code: ErrorCode.... })` for error-path assertions.
- `nodejs/tests/e2e/utils/index.ts` (helpers — `createConnection`, `connectAsync`, `destroyAsync`,
  `executeAsync`, `sleepAsync`, `getSnowflakeSDK`).
