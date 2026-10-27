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

- `auth-workload-identity-e2e.ts` — migrated to
  [`../e2e/authentication/workload-identity.test.ts`](../e2e/authentication/workload-identity.test.ts).
  `connects using transitive impersonation` is `should authenticate with WIF using impersonation`.
  AWS and GCP log in as the impersonated user on both drivers. Azure keeps the old-driver
  rejection, and the new driver logs in as the impersonated user.
  `connects using GetWebIdentityToken` landed in `a3fe4130` (2026-06-24), after this
  snapshot (`415160294a48087b18b0a70bb09534078fe4afac`, 2026-05-06), so it was not copied.
  It maps to `should authenticate AWS WIF using GetWebIdentityToken when opted in` and
  `should authenticate AWS WIF using GetWebIdentityToken when workload_identity_aws_use_outbound_token is true`.
  Both assume the issuer role and assert `CURRENT_USER()` is `TEST_WIF_E2E_AWS_WITH_ISSUER`.
  The opt-in selects `GetWebIdentityToken`; neither test observes the STS call.
- Everything under [`authentication/`](./authentication/) — migrate **one file at a time**, each
  one only when support for the corresponding auth provider (Key Pair, MFA, OAuth, Okta, PAT,
  session-token renewal) lands in the new driver.
  - [`authentication/testKeyPair.js`](./authentication/testKeyPair.js) — migrated and deleted.
    Public JWT/key-pair coverage lives in
    `nodejs/tests/e2e/authentication/private-key-auth.test.ts`.
  - [`authentication/testExternalBrowser.js`](./authentication/testExternalBrowser.js) —
    Honest counterparts in `nodejs/tests/e2e/authentication/external-browser.test.ts`:
    `Successful connection` → `should authenticate with external browser via Okta IdP`;
    `Wrong credentials` → `should fail with timeout when the IdP rejects the credentials`;
    `obtains the id token from the server and saves it on the local storage` → first
    connect of `should reuse cached ID token without browser interaction`;
    `authenticates by token, browser credentials not needed` → second connect of that
    same `it`;
    `External browser timeout` → `should fail with timeout when no browser callback arrives`
    (no token on the loopback; a real Okta tab is not required for that contract);
    `Mismatched Username` → `should fail when login request reports an IdP username mismatch`
    (same 390191; live IdP identity is not required for that contract).
    ID-token cache `it`s that Node does not read from the credential manager are
    covered in `sf_core/tests/integration/authentication/external_browser_id_token_cache.rs`
    (`should_store_id_token_after_successful_browser_login`,
    `should_authenticate_with_cached_id_token`,
    `should_retry_with_browser_flow_when_cached_id_token_fails`).
  - [`authentication/testMfa.js`](./authentication/testMfa.js) — migrated to
    `nodejs/tests/e2e/authentication/user-password-mfa.test.ts`.
  - [`authentication/testOauth.js`](./authentication/testOauth.js) — not a whole-file
    move. `Successful connection` runs as `oauth should authenticate with pre acquired access token`.
    `Mismatched username` is `should reject a mismatched username for a pre-acquired token`.
    The Node test branches on BD 98 for the wrapped message. Core
    `should_reject_a_mismatched_username_for_a_pre_acquired_token` in
    `oauth_scenarios.rs` checks that wiremock `390309` fails the connect and is not retried.
    `Invalid token` is `it.skip` (`oauth should fail legacy authentication with invalid token`).
    Python, JDBC, and ODBC skip that live negative so a junk token does not lock the
    shared account. Core ignores every OAuth e2e test behind `auth_oauth_e2e`, including
    the success tests. The skip is the disposition; that old test was not migrated.
  - [`authentication/testOauthOktaClientCredentials.js`](./authentication/testOauthOktaClientCredentials.js)
    — split between [`../e2e/authentication/oauth.test.ts`](../e2e/authentication/oauth.test.ts)
    and core. Successful connection and the invalid client ID run in the Node file.
    That test keeps the token-endpoint failure and unusable-connection assertions.
    Mismatched username is core
    `should_reject_a_mismatched_username_for_client_credentials` in `oauth_scenarios.rs`.
    Wiremock mints a token, returns `390309`, the connect fails, and the connection stays down.
  - [`authentication/testOauthSnowflakeAuthorizationCode.js`](./authentication/testOauthSnowflakeAuthorizationCode.js)
    — deleted from the snapshot. Per test:
    - `Successful connection` is `oauth should authenticate using authorization code flow`.
    - `Mismatched Username` is core
      `should_reject_a_mismatched_username_for_authorization_code` in
      `oauth_scenarios.rs`. Wiremock returns `390309`, the connect fails,
      and the connection stays down. A cached access token makes the login run
      without a browser.
    - `External browser timeout` is `should time out authorization code when browser credentials are not provided`.
      The new driver asserts the BD 95 fragment `OAuth browser authorization timed out`.
      The old exact `Browser action timed out after N ms.` string is not pinned.
    - `obtains the id token from the server and saves it on the local storage` is the first
      connect in `oauth should reuse cached access token without browser interaction`.
    - `the token is saved in the credential manager` is core
      `should_login_with_authorization_code_using_cached_refresh_token`, which asserts the
      refreshed access token and the rotated refresh token are written through `TokenCache`.
    - `authenticates by token, browser credentials not needed` is that Node test's second
      connect, whose `openExternalBrowserCallback` throws if a browser is launched. Core
      `should_login_with_authorization_code_using_cached_access_token` checks the same
      short-circuit by asserting that no IdP token request is made.
    - `opens browser again when token is incorrect` removed only the access token and left the
      refresh token. Both drivers then refresh without a browser. That is core
      `should_login_with_authorization_code_using_cached_refresh_token`.
    - `refreshes the token for credential cache key` is retired. It compared `removeFromCache`'s
      return value with the earlier access token, so it never read a refreshed token. Refresh is
      covered by core `should_login_with_authorization_code_using_cached_refresh_token`,
      `should_evict_cached_at_and_retry_via_refresh_token_on_390303`, and
      `should_evict_cached_at_and_retry_via_refresh_token_on_390318`.
  - [`authentication/testOauthOktaAuthorizationCode.js`](./authentication/testOauthOktaAuthorizationCode.js),
    [`authentication/testOauthSnowflakeWildcardsAuthorizationCode.js`](./authentication/testOauthSnowflakeWildcardsAuthorizationCode.js)
    — TODO(SNOW-4291607): implement when CI support lands for the external Okta
    authorization-code client and the wildcard Snowflake client with no redirect URI.

## Integration tests (`integration/`), sorted by priority

### Query statement API

Ensure the public API of the statement returned by `connection.execute` is fully covered:

- `integration/testStatement.js` — partially migrated. Immediate and post-completion
  `getSqlText()` behavior for successful and failed execution now lives in
  `nodejs/tests/e2e/statement.test.ts`. `with a valid token` and `with an invalid token` are
  not ported: they pass `sessionToken` to `createConnection`, which the new driver rejects
  (BD#83). The remaining statement API is still pending.
- `integration/testStreamRows.js` — pending. `Test Stream Rows HighWaterMark` asserts at most
  `highWaterMark` rows per `readable` event; the new driver pushes rows synchronously, so one
  `readable` drain can exceed it. Buffer-level backpressure (`readableLength` stops at the mark)
  is covered by `nodejs/tests/e2e/query/row-stream-high-water-mark.test.ts`.
- `integration/testUpdatedRows.js` — migrated to `nodejs/tests/e2e/query/num-updated-rows.test.ts`.

### Query binding

- `integration/testArrayBind.js` — partially migrated. The bulk single-column string stage bind
  (`testBindWithLargeArray`) moved to `nodejs/tests/e2e/query/binds.test.ts`, which covers the
  stage/CSV mechanic (upload above `arrayBindingThreshold`, CSV quoting/NULL, stage-vs-inline
  parity) for string columns. Still pending: per-type stage binding (DATE/TIME/TIMESTAMP_*/BINARY/
  BOOLEAN/VARIANT — `testArrayBind`, `testArrayBindWillNull`, `testBindWithJson`,
  `testBindWithArray`), fully-qualified table names on the stage path (`Test Array Bind - full
  path`) and cancelling an in-flight stage-bind insert (`Test Array Bind - full path with
  cancel`). The stage-upload failure fallback (`Test Array Bind Force Error on Upload file`) moved
  to `nodejs/tests/e2e/query/stage-binding-fallback.test.ts` and was deleted.
- `integration/testArrayBindCustomerTable.js` — migrated and deleted. Its single test was a
  large-scale (10000-row, 23-column) all-VARCHAR/FLOAT bulk stage bind asserting the insert
  succeeds; the stage/CSV mechanic is covered by `nodejs/tests/e2e/query/binds.test.ts` and the
  large-row-count behaviour lives in `sf_core` (shared `query/large_bindings.feature`).
- `integration/testBind.js` — partially migrated. Scalar and multi-row `?`-bind insert/select
  (`testBindWithQmark`, `testBindArrayWithQmark`) moved to the "parameter binding" describes in
  `nodejs/tests/e2e/query/data-types/{boolean,string}.test.ts` and
  `nodejs/tests/unit/query-result/binds.test.ts`. The string-column stage/CSV parity is covered by
  `nodejs/tests/e2e/query/binds.test.ts`; the `Verify stage binding and array binding` timezone
  matrix stays as a gap (TIMESTAMP_NTZ/LTZ/TZ + DATE inline-vs-stage parity across four
  timezones). Still pending: `:n` named-placeholder binding, bind error codes (`testWrongBinds`),
  statement reuse (`testBindsSameSQLWithDifferentValue`), SQL-injection safety
  (`testBindMaliciousString`), per-type binding (`testBindingWithDifferentDataType`)

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

- `integration/testConnection.js` — partially migrated. The `isValid` describe block moved to
  `nodejs/tests/e2e/connection.test.ts`; the new driver reaches the same `POST /session/heartbeat`
  through the core's `connection_heartbeat`. Still parked: `.heartbeat()` / `.heartbeatAsync()`,
  which should be removed rather than migrated — they are not part of the public documentation.
- `integration/testConnectionNegative.ts` — removed. The single generic test lives in
  `nodejs/tests/e2e/connection.test.ts` ("should reject a connection without a user name"). The
  per-option `ERR_CONN_CREATE_*` cases are not carried over: the new driver validates settings at
  connect and reports InvalidParameterError without a code (BD#101).
- `integration/testEasyLoggingOnConnecting.js` — figure out what "easy logger" is, what its public
  API surface looks like, and decide what coverage it needs.
- `integration/testManualConnection.js`:
  - `keepAlive test` — performance assertion (`sumWithoutKeepAlive * 0.66 > sumWithKeepAlive`)
    that requests run faster with `keepAlive: true`. **Validate that `sf_core` covers the same
    keep-alive performance gain**, then drop this file. There is currently no equivalent
    integration test in `_old-driver-reference/` (`unit/snowflake_config_test.js` only validates
    the config knob, not the runtime behavior).
  - `Connection file configuration test` — password and default-profile cases are covered by
    `nodejs/tests/e2e/session/connection-config-file.test.ts`. The oauth name variants stay here.
  - `test - connect CLIENT CREDENTIALS - Snowflake` and
    `test - connect CLIENT CREDENTIALS - inconsistent username` are deferred. They are
    manual runs of connect plus SELECT 1, and of the mismatch sentence. They add no
    unique automated OAuth contract coverage. The same quick manual runs can be migrated later.


### PUT / GET

Migration Done. Most cases covered by core / common Gherkin cases. The status of migration for old
driver tests is [here](https://docs.google.com/spreadsheets/d/1AcEsioMFLa6Y2FIAxpgMR3cY2DBEtz1Z_qAqrOq44pA/edit?usp=sharing)

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

- `integration/testLoginRequestBody.ts` — migrated to `nodejs/tests/e2e/login-request-body.test.ts`;
  the runnable cases are replaced by a comment in the source file. `APPLICATION`, `APPLICATION_PATH`,
  and OS_DETAILS on Linux run on both drivers, and ISA runs
  on both with BD#81. Skipped for the new driver as not implemented:
  `LIBC_FAMILY` / `LIBC_VERSION`, and `OS_DETAILS: null` on non-Linux platforms. `PLATFORM` (BD#70) is not ported;
  `sf_core` covers it in `src/telemetry/platform_detection/tests.rs` and
  `tests/integration/telemetry/platform_detection.rs`.
- `integration/testProxyExecute.js` — proxy logic should be covered in `sf_core`. All we care
  about in the driver tests is that when we pass a proxy config to a connection, `sf_core` accepts
  it. Park until after the beta release.
- `integration/testRequestParams.js` — verify that `sf_core` attaches a GUID to outgoing requests.
  Ensure all 4 paths covered by the original `snowflake-connector-nodejs` test are covered there.

### Wiremock — auth providers

Each of these depends on the corresponding auth provider landing in `sf_core` + the new driver:

- `integration/wiremock/testOauthAuthorizationCode.js` — still in the snapshot. Authorization
  Code is implemented, and the file is not migrated:
  - Successful flow: live `oauth should authenticate using authorization code flow` plus core
    `should_login_with_authorization_code_through_injected_opener`.
  - Empty scope: no assertion that the authorize URL omits `scope`. Core
    `should_omit_oauth_scope_when_scope_and_role_are_absent` only checks the resolver.
  - Redirect `invalid_scope`, invalid state, HTTP 400 invalid code, missing token, and occupied
    port: no authorization-code test preserves those assertions. Unit `OAuthError::StateMismatch`
    exists; the Node outer error and the old port-unavailable message are not covered.
  - Single-use refresh: the old test only checks that connect does not throw. Core
    `should_omit_single_use_refresh_flag_from_refresh_grant_body` checks the refresh body, which
    that test did not.
- `integration/wiremock/testOauthClientCredentials.js` — the four cases are covered
  separately. Successful connection and `SELECT 1` are the live Node test. Empty scope is
  `should omit scope from the client credentials token request when role and scope are unset`
  in [`../e2e/authentication/oauth.test.ts`](../e2e/authentication/oauth.test.ts), which
  connects and checks the token body has no `scope=`. HTTP 400 `invalid_client` is the Node test
  `should fail client credentials when the token endpoint returns invalid_client`, which
  returns that old mapping's 400 body. A missing `access_token` is the core
  integration test plus the Node tests that branch on BD 138. The kept `OAUTH_TYPE`
  assertion checks `data.OAUTH_TYPE`, not the old `data.CLIENT_ENVIRONMENT.OAUTH_TYPE`
  matcher.
- `integration/wiremock/testOauthPat.js` — migrate once PAT is implemented in `sf_core` and the
  new Node driver.
- `integration/wiremock/testOauthRefreshToken.js` — once all auth providers are in `sf_core`,
  verify this case is covered there.
- `integration/wiremock/testExternalBrowserSsoUrlError.ts` — runnable case replaced by the
  comment in that file pointing at `nodejs/tests/e2e/authentication/external-browser.test.ts`
  (`should fail when authenticator-request reports SSO URL generation failure`).
- `integration/wiremock/testExternalBrowser.ts` — TODO: migrate `successfully logs in with custom
  browserResponseRenderer` when `browserResponseRenderer` is implemented. The same test also
  honestly covers fixed `browserRedirectPort` behavior by sending the callback to that port.
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

- Automated coverage of `.toml`-driven connection creation lives in
  `nodejs/tests/e2e/session/connection-config-file.test.ts`
  (`tests/definitions/shared/session/connection_config_file.feature`). The
  `Connection file configuration test` block of
  [`integration/testManualConnection.js`](./integration/testManualConnection.js) stays
  manual-only for the `aws-oauth`, `aws-oauth-accessUrl`, and `aws-oauth-file` variants.

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
  Partially migrated: `oauth without username` splits. Login with the user omitted
  runs as `should authenticate with pre acquired access token without user`.
  The old row also asserted `accessUrl` `https://account.snowflakecomputing.com`
  and `account` `account`. This wrapper does not build that URL; core derives the
  server URL from `account` / `host`. `should forward legacy OAUTH without a username`
  is the Node boundary: `account`, `authenticator`, and `token` are forwarded, and
  a username is not required. The `accessUrl` assertion is retired with
  `ConnectionConfig`. The `invalid username with OAUTH authenticator` and
  `invalid oauth token` type-check rows stay; `toCoreConnectionOptions`
  currently `String()`s those values.
  The two `oauth public client parameters` rows (`.com` and `.cn`) assert the filled
  config: client id and secret `LOCAL_APPLICATION`, and
  `http://{host}:8082/oauth/authorize` plus `http://{host}:8082/oauth/token-request`.
  The shared authorization-code login leaves both URL options unset, as the other wrappers
  do, and checks that login succeeds. It does not assert those filled values.
  `LOCAL_APPLICATION` is
  `resolve_client_credentials_substitutes_local_application_for_snowflake_idp`.
  The path suffix is `resolve_default_urls_use_server_host`, which builds
  `https://{host}/oauth/authorize` and `https://{host}/oauth/token-request` and drops the
  connection protocol and port. The `http://{host}:{port}` form stays unmapped.
  - JWT-named negatives (`missing/invalid username with SNOWFLAKE_JWT`, `invalid private key` /
    `privateKeyPath` / `privateKeyPass`) assert `ERR_CONN_CREATE_*` on `new ConnectionConfig`.
    The new driver reports these at connect as InvalidParameterError without a code (BD#101), so
    no case here migrates as written.
- `unit/connection/normalize_connection_options_test.ts` — migrated and deleted. Its cases live in
  `nodejs/tests/unit/normalize-connection-options.test.ts`, with
  `nodejs/tests/e2e/normalize-connection-options.test.ts` connecting through the normalized options.
  - `private_key_file` → `privateKeyPath` (and camel / `private_key_pass`) mapping is covered by
    public `createConnection` in `nodejs/tests/e2e/authentication/private-key-auth.test.ts`.
    Keep the production `privateKeyPath` alias; do not add a normalizer-only test for it.
- [`unit/connection/statement_test.js`](./unit/connection/statement_test.js) — covers error paths
  in the query-execution API. The new driver exposes a smaller set of error codes, so review
  after beta against the final error taxonomy.
- [`unit/connection/result/`](./unit/connection/result/) — park until after the beta release,
  then check whether the new driver's result-handling coverage has gaps that these tests
  highlight. TIME-related files, against the TIME e2e in `query-data-types.test.ts`:
  - [`datetime_format_converter_test.js`](./unit/connection/result/datetime_format_converter_test.js) —
    dropped. Token coverage lives in `sf_output_format` unit tests;
    [`time.test.ts`](../e2e/query/data-types/time.test.ts) (`TIME_OUTPUT_FORMAT`) wires
    session `TIME_OUTPUT_FORMAT` (bare `FF`, `FFn`, date tokens, `TZH:TZM`).
    Timestamp-only moment mapping is not a TIME contract.
  - [`sf_timestamp_test.js`](./unit/connection/result/sf_timestamp_test.js) — `Time: basic`
    dropped; scale/`FF` wiring is in `time.test.ts`. `Timestamp: basic` lives in
    `nodejs_bridge` `date_format.rs` and the timestamp e2e files
    (`timestamp-ntz.test.ts`, `timestamp-ltz.test.ts`, `timestamp-tz.test.ts`).
    `Date: basic` stays until DATE cells are `SnowflakeDate`.
  - [`result_test_variant.js`](./unit/connection/result/result_test_variant.js) — JSON and XML
    parsing is covered by
    [`semi-structured.test.ts`](../e2e/query/data-types/semi-structured.test.ts). The new driver
    has no `new Function()` fallback, warning, or telemetry for values that are not standard JSON
    (BD#140), so the fallback test has no counterpart. Drop the file after beta.

### Configuration / global config

- [`unit/configuration/`](./unit/configuration/) — appears to test dead code that is not
  reachable from the public API. Confirm after beta and drop if so.
- [`unit/global_config_test.ts`](./unit/global_config_test.ts) — migrate as soon as the new
  driver exposes a global-config surface.
- [`unit/snowflake_config_test.js`](./unit/snowflake_config_test.js) — `logLevel` cases moved to
  `nodejs/tests/unit/logging/configure.test.ts`. After beta, ensure there is e2e coverage for
  every remaining option of `snowflake.configure`. This file does not cover all options, so use
  it as a checklist rather than a 1:1 migration source.

### Snowflake top-level API

- [`unit/snowflake_test.js`](./unit/snowflake_test.js) — useful scenarios that belong as e2e
  tests in the new driver. After beta, map each case to existing e2e tests and fill the gaps.
  The `getSqlText()` cases for execute success, execute failure, and `fetchResult()` now live
  in `nodejs/tests/e2e/statement.test.ts`; request-ID resubmission is still pending.
  The `connect()` refusal for a fatally disconnected connection now lives in
  `nodejs/tests/e2e/connection-state-errors.test.ts`.
  The `connect()` refusal while a login is in flight now lives in
  `nodejs/tests/e2e/connection-state-errors.test.ts`.
  The `connection.destroy()` refusals in the pristine and disconnected states, and destroy
  while a login is in flight, now live in `nodejs/tests/e2e/connection-state-errors.test.ts`.

### Authentication

- `unit/authentication/auth_workload_identity/` — pushed down to the WIF tests in
  [`../../../sf_core/src/rest/snowflake/workload_identity/`](../../../sf_core/src/rest/snowflake/workload_identity/)
  and migrated to
  [`../e2e/authentication/workload-identity.test.ts`](../e2e/authentication/workload-identity.test.ts).
  Azure `throws error when impersonation path is provided` is the live
  `should authenticate with WIF using impersonation`. AWS and GCP log in as the impersonated
  user on both drivers. Azure keeps the old-driver rejection, and the new driver logs in as
  the impersonated user.
- The WIF cases in `unit/connection/normalize_connection_options_test.ts` — migrated to
  [`../unit/normalize-connection-options.test.ts`](../unit/normalize-connection-options.test.ts).
  `getAuthenticator()` row `workload identity` → `AuthWorkloadIdentity` is retired with that
  class. The other rows in that table, except `unknown`, named removed Node authenticator
  classes and are retired with them. `unknown` stays: it is the only assertion that an
  unrecognized authenticator falls back to `AuthDefault`.
- `unit/authentication/custom_credential_manager_test.js` — migrated to
  `nodejs/tests/{unit/global-config,e2e/authentication/custom-credential-manager}.test.ts` and
  deleted.
- [`unit/authentication/`](./unit/authentication/) — defer until after the beta release and
  revisit when implementing each auth provider, the same way the integration `authentication/`
  folder is handled.
  - The `key-pair authentication` describe in
    [`unit/authentication/authentication_test.js`](./unit/authentication/authentication_test.js)
    was removed. JWT token assembly is `sf_core` (`generate_jwt_token`, private-key e2e, login
    `AUTHENTICATOR=SNOWFLAKE_JWT`). Public Node options live in
    `nodejs/tests/e2e/authentication/private-key-auth.test.ts`. Thenable
    `AuthKeypair.authenticate` / `reauthenticate` overwriting `TOKEN` are not ported.
  Partially migrated in `unit/authentication/authentication_test.js`:
  `oauth - authenticate method is thenable` is retired. `AuthOauth` no longer
  exists, so there is no thenable `authenticate` method to call.
  `oauth - check token` and `oauth - check authenticator` split. Node forwards
  the strings in `should map the access token used by legacy OAUTH and
  WORKLOAD_IDENTITY with an OIDC provider`. The login JSON is
  `sf_core/tests/integration/authentication/oauth.rs`
  `should_login_with_legacy_oauth_using_pre_acquired_token`, which never calls
  Node `createConnection`.
  `getAuthenticator()` table row `oauth` → `AuthOauth` is retired with that
  class. The public authenticator is the string that unit test forwards.

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
- [`unit/large_result_set/testLargeResultSet.js`](./unit/large_result_set/testLargeResultSet.js) —
  partially covered by `tests/e2e/query/large-result-set-errors.test.ts` (LargeResultSetError
  for an error status and for a 200 response with an XML body, NetworkError for a reset
  connection). The XML case is skipped for the new driver until SNOW-4334410. Not covered yet: a
  503 or a hang that exhausts retries, and recovery on the last retry.

### Disk cache

- [`unit/disk_cache_test.ts`](./unit/disk_cache_test.ts) — after beta, check whether `sf_core`
  writes to the same cache folders the new Node driver expects. If not, add coverage that the
  paths are configurable on the `sf_core` side and that the driver passes them through.

### Logger / easy logging

- [`unit/logger/node_test.js`](./unit/logger/node_test.js) — file JSON format, `logLevel` (including
  TRACE as DEBUG), and console/file sinks now live in
  `nodejs/tests/unit/logging/configure.test.ts` and `logging.test.ts`. Remaining coverage here is
  Winston-only (`includeTimestamp: false`, distinct TRACE lines) and is not ported.
- [`unit/logger/easy_logging_starter_test.js`](./unit/logger/easy_logging_starter_test.js) —
  after beta, research what "easy logging" is, its public API, and decide what coverage it
  needs. Pairs with `integration/testEasyLoggingOnConnecting.js`.

### Telemetry

- `unit/telemetry/application_path_test.ts` — removed. `src/wrapper-identity.ts` matches the old
  `require.main.filename`, then `process.argv[1]` order, and `e2e/login-request-body.test.ts`
  checks the login body. The empty case is the `sf_core` executable-path fallback.
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
