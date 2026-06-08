# Old Driver Test Migration Plan

This document describes our plan for migrating each test file from the legacy
[`snowflake-connector-nodejs`](https://github.com/snowflakedb/snowflake-connector-nodejs) test
suite into this repository, and **when** each file is expected to be tackled.

The high-level migration workflow lives in
[`README.MD`](./README.MD) and the per-file migration procedure lives in
[`.cursor/commands/migrate-nodejs-test.md`](../../../.cursor/commands/migrate-nodejs-test.md).
This file is the bookkeeping layer on top of those: one row per source file, with a concrete
action plan and ordering.

Each test file ends up in one of these buckets:

- **Migrate to `tests/e2e/`** — covers a public driver method end-to-end. Use the
  `/migrate-nodejs-test` command.
- **Push down to `sf_core`** — exercises internals, edge cases, parameter matrices, or anything
  that's really logic living in Rust. The corresponding coverage should be added (or verified) in
  `sf_core` tests, and the legacy file deleted.
- **Drop** — already covered elsewhere, asserts a server property rather than a driver property,
  or is otherwise obsolete (logger-only, internal mocking, etc.).
- **Defer** — covered last, once the underlying feature support lands in the new driver. Tracked
  explicitly below.

Deferred work — covered **last**, only once the corresponding feature support lands:

- [`auth-workload-identity-e2e.ts`](./auth-workload-identity-e2e.ts) — migrate when we add
  Workload Identity Federation (WIF) support.
- Everything under [`authentication/`](./authentication/) — migrate **one file at a time**, each
  one only when support for the corresponding auth provider (External Browser, Key Pair, MFA,
  OAuth, Okta, PAT, session-token renewal) lands in the new driver.

## Integration tests (`integration/`), sorted by priority

### Query statement API

Ensure the public API of the statement returned by `connection.execute` is fully covered:

- `integration/testStatement.js`
- `integration/testStreamRows.js`
- `integration/testUpdatedRows.js`

### Query binding

- `integration/testArrayBind.js` — _TBD_
- `integration/testArrayBindCustomerTable.js` — _TBD_
- `integration/testBind.js` — _TBD_

### Query execution

- `integration/testExecute.js` — contains tests that look like duplicates of other query-execution
  coverage. Re-review once we have full query-execution coverage in the new driver.
- `integration/testLargeResultSet.js` — from the Node.js side a "large" result set is no different
  from any other query. Reconsider deleting this file once query execution is implemented in the
  new driver.

### Structured types

Review when working on structured-types support in the new driver:

- `integration/testDataType.js` — 2 leftover tests related to structured types.
- `integration/testStructuredType.js`

### Connection

- `integration/testConnection.js` — park `heartbeat` / `isValid` coverage for now. When
  implementing the connection surface in the new driver:
  - `.heartbeat()` / `.heartbeatAsync()` should be removed — they are not part of the public
    documentation.
  - `.isValidAsync()` — verify that `sf_core` implements this correctly before migrating coverage.
- `integration/testConnectionNegative.ts` — once connection creation lands in the new driver,
  rewrite this as a single test that asserts a generic "invalid connection parameters" error code.
- `integration/testEasyLoggingOnConnecting.js` — figure out what "easy logger" is, what its public
  API surface looks like, and decide what coverage it needs.
- `integration/testManualConnection.js`:
  - `keepAlive test` — performance assertion (`sumWithoutKeepAlive * 0.66 > sumWithKeepAlive`)
    that requests run faster with `keepAlive: true`. **Validate that `sf_core` covers the same
    keep-alive performance gain**, then drop this file. There is currently no equivalent
    integration test in `_old-driver-reference/` (`unit/snowflake_config_test.js` only validates
    the config knob, not the runtime behavior).
  - `Connection file configuration test` — see "Missing coverage" below; this is the only existing
    coverage for `.toml`-driven connection creation and should drive the e2e migration of that
    surface.


### PUT / GET

Review these files together. We want a much smaller suite (most logic belongs in `sf_core`), but
it must still cover the different API usages — wildcards, streaming vs. regular execution, etc.

- `integration/testPutGet.js`
- `integration/testPutSmallFiles.js`

### OCSP

OCSP-related tests will be migrated only once we are 100% sure OCSP support is needed in the new
driver:

- `integration/ocsp_mock/` (folder — `https_ocsp_mock_agent.js`, `testConnectionOcspMock.js`)
- `integration/testConnectionWithOCSP.js`
- `integration/testOcsp.js`

### CRL

When implementing the CRL API in the new driver, ensure `sf_core` has coverage for what's in
`integration/testCrl.ts` and `unit/agent/crl_validator/`. E2E should keep only a single
sanity-check test that validates CRL works (and ideally one that validates CRL through a proxy).

- `integration/testCrl.ts`

### Login / proxy / request plumbing

- `integration/testLoginRequestBody.ts` — most of this is `sf_core`-specific. We only need a test
  that verifies `APPLICATION_PATH` is correctly passed through. Park until after the beta release.
- `integration/testProxyExecute.js` — proxy logic should be covered in `sf_core`. All we care
  about in the driver tests is that when we pass a proxy config to a connection, `sf_core` accepts
  it. Park until after the beta release.
- `integration/testRequestParams.js` — verify that `sf_core` attaches a GUID to outgoing requests.
  Ensure all 4 paths covered by the original `snowflake-connector-nodejs` test are covered there.

### Wiremock — auth providers

Each of these depends on the corresponding auth provider landing in `sf_core` + the new driver:

- `integration/wiremock/testOauthAuthorizationCode.js` — migrate once Authorization Code is
  implemented in `sf_core` and the new Node driver.
- `integration/wiremock/testOauthClientCredentials.js` — migrate once Client Credentials is
  implemented in `sf_core` and the new Node driver.
- `integration/wiremock/testOauthPat.js` — migrate once PAT is implemented in `sf_core` and the
  new Node driver.
- `integration/wiremock/testOauthRefreshToken.js` — once all auth providers are in `sf_core`,
  verify this case is covered there.
- `integration/wiremock/testExternalBrowserSsoUrlError.ts` — once all auth providers are in
  `sf_core`, verify this case is covered there.
- `integration/wiremock/testPoolAuthCoordination.ts` — migrate once all auth providers **and** the
  connection pool are implemented in the new driver.

### Wiremock — networking / sessions

- `integration/wiremock/testRequestRetry.ts` — once we are in beta, validate that `sf_core` has
  coverage for this case.
- `integration/wiremock/testSessionTokenRenewal.ts` — once we are in beta, validate that `sf_core`
  has coverage for this case.

### Drop — not driver-level coverage

These were effectively unit tests against driver internals. Check where they're used and confirm
`sf_core` has equivalent coverage; otherwise migrate the behavior into `sf_core`.

- `integration/testCache.js`
- `integration/testEncrypt.js`
- `integration/testHTAP.ts` — query-context-cache coverage; needs `sf_core` coverage.
- `integration/wiremock/testStatementReceivingMalformedResponse.ts` — written under the false
  assumption that the backend was returning malformed responses; the real issue turned out to be
  that the Node connection was created from Go-driver session tokens. All we need in the new
  driver is a negative test that ensures a query-execution error is propagated to the callback.
- `integration/testMaxLobSize.js` — manual-only test that ensures large-LOB inserts don't crash
  the driver. Remove this file once we're confident the new driver's performance tests cover the
  same scenarios.

### Shared helpers

Drop each of these once no remaining old test file imports it:

- `integration/connectionOptions.js`
- `integration/sharedLogger.js`
- `integration/sharedStatements.js`
- `integration/testUtil.js`
- `integration/test_utils/` (folder — `httpInterceptorUtils.js`)

### Missing coverage

- The only existing coverage of `.toml`-driven connection creation lives in the
  `Connection file configuration test` block of
  [`integration/testManualConnection.js`](./integration/testManualConnection.js) (manual-only,
  gated behind `RUN_MANUAL_TESTS_ONLY`). It exercises `snowflake.createConnection(null)` and
  `snowflake.createPool(null, …)` against `connections.toml` with `SNOWFLAKE_HOME` /
  `SNOWFLAKE_DEFAULT_CONNECTION_NAME`, including the `aws-oauth`, `aws-oauth-accessUrl`, and
  `aws-oauth-file` connection-name variants. Migrate this to an automated e2e test when
  `.toml`-driven connection creation lands in the new driver.

## Unit tests (`unit/`)

Most of these exercise driver internals rather than public surface, so the default disposition is
**push down to `sf_core`** (verify equivalent coverage exists there, then drop the legacy file).
The exceptions — files that should be migrated into the new driver's own unit/e2e suite — are
called out explicitly.

### Connection

- [`unit/connection/connection_config_test.js`](./unit/connection/connection_config_test.js) —
  park until after the beta release. `sf_core` + the new Node driver will surface a different set
  of errors when `.createConnection` fails, so this needs a re-review against the new error
  taxonomy rather than a line-by-line migration.
- [`unit/connection/normalize_connection_options_test.ts`](./unit/connection/normalize_connection_options_test.ts) —
  migrate once `normalizeConnectionOptions` is implemented in the new driver.
- [`unit/connection/statement_test.js`](./unit/connection/statement_test.js) — covers error paths
  in the query-execution API. The new driver exposes a smaller set of error codes, so review
  after beta against the final error taxonomy.
- [`unit/connection/result/`](./unit/connection/result/) — park until after the beta release,
  then check whether the new driver's result-handling coverage has gaps that these tests
  highlight.

### Configuration / global config

- [`unit/configuration/`](./unit/configuration/) — appears to test dead code that is not
  reachable from the public API. Confirm after beta and drop if so.
- [`unit/global_config_test.ts`](./unit/global_config_test.ts) — migrate as soon as the new
  driver exposes a global-config surface.
- [`unit/snowflake_config_test.js`](./unit/snowflake_config_test.js) — after beta, ensure there
  is e2e coverage for every option of `snowflake.configure`. This file does not cover all
  options, so use it as a checklist rather than a 1:1 migration source.

### Snowflake top-level API

- [`unit/snowflake_test.js`](./unit/snowflake_test.js) — useful scenarios that belong as e2e
  tests in the new driver. After beta, map each case to existing e2e tests and fill the gaps.

### Authentication

- [`unit/authentication/`](./unit/authentication/) — defer until after the beta release and
  revisit when implementing each auth provider, the same way the integration `authentication/`
  folder is handled.

### Agent (CRL / OCSP)

- [`unit/agent/crl_validator/`](./unit/agent/crl_validator/) — surfaces missing CRL coverage in
  `sf_core`. Push that coverage down to `sf_core`, then drop. Tracked alongside the CRL section
  in the integration plan above.
- [`unit/agent/ocsp_response_cache_test.ts`](./unit/agent/ocsp_response_cache_test.ts) — migrate
  only once we are 100% sure OCSP support is needed in the new driver (same gate as the
  integration OCSP files).
- [`unit/ocsp/`](./unit/ocsp/) — same gate as above; migrate only once OCSP support is confirmed
  for the new driver.

### HTTP / proxy / request plumbing

- [`unit/http/`](./unit/http/) — after beta, verify `sf_core` has equivalent coverage and drop.
- [`unit/proxy_util_test.js`](./unit/proxy_util_test.js) — verify `sf_core` has equivalent
  coverage and drop. Pairs with `integration/testProxyExecute.js`.
- [`unit/query_context_cache_test.js`](./unit/query_context_cache_test.js) — verify `sf_core`
  has equivalent QCC coverage and drop. Pairs with `integration/testHTAP.ts`.

### File transfer / large result sets

- [`unit/file_transfer_agent/`](./unit/file_transfer_agent/) — after beta, verify `sf_core`
  covers everything tested here. PUT/GET logic lives in `sf_core` going forward.
- [`unit/large_result_set/`](./unit/large_result_set/) — appears redundant with other
  result-handling coverage. After beta, confirm and drop.

### Disk cache

- [`unit/disk_cache_test.ts`](./unit/disk_cache_test.ts) — after beta, check whether `sf_core`
  writes to the same cache folders the new Node driver expects. If not, add coverage that the
  paths are configurable on the `sf_core` side and that the driver passes them through.

### Logger / easy logging

- [`unit/logger/node_test.js`](./unit/logger/node_test.js) — migrate as soon as the new driver
  has a logger, to cover its configuration surface.
- [`unit/logger/easy_logging_starter_test.js`](./unit/logger/easy_logging_starter_test.js) —
  after beta, research what "easy logging" is, its public API, and decide what coverage it
  needs. Pairs with `integration/testEasyLoggingOnConnecting.js`.

### Telemetry

- [`unit/telemetry/application_path_test.ts`](./unit/telemetry/application_path_test.ts) —
  migrate the application-name fetching coverage once the new driver is implemented.
- [`unit/telemetry/platform_detection_test.ts`](./unit/telemetry/platform_detection_test.ts) —
  must be implemented in `sf_core` and validated that the implementation matches what this
  test asserts.

### Mocks / streaming

`unit/mock/` contains test utilities plus two test files:

- [`unit/mock/statement_fetch_as_string.js`](./unit/mock/statement_fetch_as_string.js) — after
  beta, ensure this is covered by both unit and e2e tests.
- [`unit/mock/statement_stream_result.js`](./unit/mock/statement_stream_result.js) — appears to
  duplicate the streaming integration tests. After beta, confirm the streaming integration
  coverage is sufficient and drop.

### Miscellaneous

- [`unit/errors_test.ts`](./unit/errors_test.ts) — unclear whether this is still needed.
  Migrate to the new driver only if a concrete need surfaces.
- [`unit/libc_details_test.ts`](./unit/libc_details_test.ts) — must be implemented in `sf_core`
  and covered there.
- [`unit/util_test.js`](./unit/util_test.js) — after beta, validate which utilities are still
  needed in `sf_core` and drop.
