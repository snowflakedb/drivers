# Changelog

## Upcoming Beta Release

### New Architecture

Version 4 is a full rewrite. Most of the code is now written in Rust and compiled into a native `.node` addon, which makes the driver much faster. The public API is the same, apart from the breaking changes listed below. Since this is a new codebase, expect some small differences in behavior, including different error messages and codes. If your code checks error text, test it before you upgrade.

npm installs the prebuilt addon for your platform as an optional dependency, `@snowflake/sdk-core-<platform>`. We ship builds for:

- macOS: arm64, x64
- Linux: arm64, x64 (glibc and musl)
- Windows: arm64, x64, x86

You may need to adjust your setup if you:

- install with `--omit=optional`. The addon won't be installed.
- run `npm install` on one platform and deploy to another, for example from macOS to a Linux container or AWS Lambda. Run `npm install` on the target platform instead.
- bundle your app with webpack, esbuild or a similar tool. Mark `snowflake-sdk` as external.
- use a platform that isn't listed above. You'll need to build the addon from source with Rust.

### Known Issues

> These issues will be addressed in upcoming beta releases, before the official 4.x release.

- FIPS mode is not supported yet. The Node.js FIPS settings, such as the `--enable-fips` and `--force-fips` flags or `crypto.setFips()`, only change how the built-in `crypto` module works. The new driver does its cryptography in its native library instead of that module, so these settings have no effect on it.

### Breaking Changes

> [!WARNING]
> This list of breaking changes is not final. We may add or remove entries based on feedback from the beta.

- Changed the minimum supported Node.js version from 20 to 22, because Node.js 20 has reached end of life. Upgrade to Node.js 22 or later before you update the driver.
- The driver no longer honors most Node.js runtime flags, environment variables and global settings of the Node.js `tls`, `https`, `dns` or `crypto` modules, because it doesn't use these modules. For example, flags such as `--use-openssl-ca`, `--openssl-config`, `--tls-min-v1.3`, `--tls-cipher-list` or `--dns-result-order`, and code such as `tls.DEFAULT_MIN_VERSION = 'TLSv1.3'` or `dns.setDefaultResultOrder('ipv4first')`, have no effect on it. Only two Node.js environment variables still work:
  - `NODE_TLS_REJECT_UNAUTHORIZED=0` turns off TLS certificate checks.
  - `NODE_EXTRA_CA_CERTS` adds the certificates from the given file to the list of trusted certificates.
- Removed OCSP checks. Snowflake drivers now check for revoked certificates with CRLs (certificate revocation lists), because OCSP often caused connection failures and many certificate authorities are dropping it. These checks are off by default, as OCSP checks have been since version 3.4.0. To turn them on, set the `certRevocationCheckMode` connection option to `ENABLED` (in place of OCSP fail-closed mode) or `ADVISORY` (in place of fail-open mode). These APIs were removed:
  - The `disableOCSPChecks` and `ocspFailOpen` options of `snowflake.configure()`.
  - The `useConnectionConfigProxyForOCSP` connection option.
  - The `connection.setupOcspPrivateLink()` method.
  - The `snowflake.ocspModes` export and the `OcspModes` type.
- Changed `connection.connectAsync()` to report every error by rejecting the returned promise, and removed its callback parameter. Before, errors went to two different places: login errors, such as a wrong password, went only to the callback while the promise still resolved, and setup errors, such as an invalid `clientConfigFile`, rejected the promise. Handle all errors with `try`/`catch` around `await connection.connectAsync()` or with `.catch()`.
- Data Types
  - Changed `TIMESTAMP_TZ`, `TIMESTAMP_LTZ` and `TIME` values with a zero UTC offset to show the offset as `Z` instead of `+0000`, `+00:00` or `+00` when they are returned as text, for example `2024-01-15 10:30:00.000 Z`. This now matches the text that Snowflake returns for `TO_CHAR` and `::VARCHAR`. For timestamps, this applies to `fetchAsString: ['Date']` and `toJSON()`, so also to `JSON.stringify()` of rows. `TIME` values are always returned as text, so for them it applies whenever `TIME_OUTPUT_FORMAT` contains `TZH`. `new Date()` reads both forms the same way. If your code checks the offset text with a regex or a strict format, also accept `Z`.
  - Changed `VARIANT`, `OBJECT`, `ARRAY` and `MAP` values to be parsed with `JSON.parse()` only. Before, the driver evaluated values that were not valid JSON as JavaScript code. We removed this as a safety precaution and because `JSON.parse()` is much faster.
    By default (the `STRICT_JSON_OUTPUT` parameter is `FALSE`), Snowflake can return `NaN`, `Infinity` and `undefined` in these values, and they are not valid JSON. For example, `OBJECT_CONSTRUCT('a', 'NaN'::FLOAT)` returns `{"a": NaN}`, and `ARRAY_CONSTRUCT(1, NULL, 2)` returns `[1, undefined, 2]`. Queries that return such values now fail with an error. To read them, either:
    - set `STRICT_JSON_OUTPUT` to `TRUE`. Snowflake then returns `null` instead of `undefined`, and the strings `"NaN"` and `"Infinity"`.
    - pass your own `jsonColumnVariantParser` to `snowflake.configure()`.
  - Changed integer columns to return a native JavaScript `BigInt` instead of a `big-integer` object when the `jsTreatIntegerAsBigInt` connection option or the `JS_TREAT_INTEGER_AS_BIGINT` session parameter is enabled. `BigInt` is built into JavaScript and needs no extra package. Both types hold the same values and work mostly the same way. The difference is that `big-integer` adds its own helper methods on top. If you use them, for example `.equals()`, `.add()` or `BigInteger.isInstance()`, switch to `BigInt` operators such as `===` and `+`, and to `typeof value === 'bigint'`.
  - Changed `BINARY` columns to return a plain `Buffer`. Before, the driver added two undocumented methods, `toStringSf()` and `getFormat()`, to every `Buffer` it returned. These methods were removed. To get the value as text, use `buffer.toString('hex').toUpperCase()` or `buffer.toString('base64')`.
  - Changed `VECTOR` columns to return an array of numbers, such as `[1.5, 2.5, 3.5]`, instead of the text sent by the server, such as `'[1.500000,2.500000,3.500000]'`. If your code parses that text, use the array directly. To get a string instead, add `'Number'` to `fetchAsString`: the array is then returned as JSON text, such as `'[1.5,2.5,3.5]'`, and a NULL value as `'NULL'`, the same as for other numbers.
- Connection Options
  - Changed the `clientStoreTemporaryCredential` connection option to default to `true`. External browser and OAuth authorization code logins now cache their tokens unless you set it to `false`, so users are asked to log in less often. To keep the old behavior, set `clientStoreTemporaryCredential: false`.
  - Removed the `clientRequestMFAToken` connection option. MFA token caching is now controlled by `clientStoreTemporaryCredential`, the same option that controls caching for external browser and OAuth logins. Because that option defaults to `true`, MFA tokens are now cached by default when your account allows it. To turn MFA token caching off, set `clientStoreTemporaryCredential: false`.
  - Changed connecting to fail with an `InvalidParameterError` when `authenticator` is not a supported value, for example because of a typo. Before, the driver logged a warning and fell back to password login, so the typo could go unnoticed when a `password` was also set.
  - Changed connecting to fail with an `InvalidParameterError` when `workloadIdentityProvider`, `workloadIdentityAzureEntraIdResource`, `workloadIdentityImpersonationPath` or `workloadIdentityAwsUseOutboundToken` is set but `authenticator` is not `WORKLOAD_IDENTITY`. Before, the driver ignored these options and logged in with the other authenticator, which hid a mistake in the connection settings. Remove these options, or set `authenticator` to `WORKLOAD_IDENTITY`.
  - Removed the `disableConsoleLogin` connection option. It is no longer needed because Snowflake already chooses the right login page. External browser login always opens the URL that Snowflake returns, as it did with the default value `true`:
    - If the account has the `ENABLE_IDENTIFIER_FIRST_LOGIN` parameter turned on, Snowflake returns its login page, where the user picks one of the account's identity providers.
    - Otherwise the account can have only one identity provider, and the browser goes straight to it.
  - Removed the `oauthHttpAllowed` connection option. It was deprecated and meant only for tests. `oauthTokenRequestUrl` must still use `https://`, but `http://` is now accepted without any option when the host is `localhost`, `127.0.0.1` or `::1`, for example for a local mock server. Remove `oauthHttpAllowed` from your connection options.
- Query Execution
  - Changed `connection.execute()` to fail with a `ClientError` with code `407001` when it is called while the connection is still logging in. Before, the driver held such queries and ran them once the login finished. The new driver doesn't hold queries. Wait until the connection is established before you run queries, for example with `await connection.connectAsync()` or inside the `connect()` callback.
  - Changed the `complete` callback of `connection.execute()` to receive `undefined` as its `rows` argument when `asyncExec` is `true`. Before, the driver put a `{ queryId }` object in `rows` by mistake: `rows` is typed as an array of rows or `undefined`, and an async query has no rows yet. The query ID has always been available from `statement.getQueryId()`, so use that instead.
  - Changed `connection.execute()` with `describeOnly: true` to fail for a multi-statement query. Before, it succeeded but returned only one placeholder column named `multiple statement execution`, not the columns of your statements, because Snowflake doesn't describe the statements of a multi-statement query. To get column metadata, run `describeOnly` on each statement separately.
  - Removed the `FileAndStageBindStatement` type and moved its `hasNext()` and `NextResult()` methods to `RowStatement`, which every query returns. These methods step through the results of a multi-statement query and have nothing to do with files or stages, so declaring them only on `FileAndStageBindStatement` was a mistake in the types. For a single statement, `hasNext()` returns `false`. If your code uses `FileAndStageBindStatement`, replace it with `RowStatement`.
  - Changed `GET` to fail when no file in the stage matches the path or `PATTERN`. Before, it returned an empty result and downloaded nothing, which also hid a wrong path. If a `GET` in your code can match no files, catch the error, or run `LIST` with the same path and `PATTERN` first.
  - Removed support for `SOURCE_COMPRESSION=BR` in `PUT`. Use `BROTLI`, the name in the Snowflake docs, which now works. Before, `BR` was the only way to upload Brotli files, because `BROTLI` failed.
- Changed `connection.isUp()` to return `false` only when the session has ended: the login expired, or Snowflake reports that the session is closed or no longer exists. Before, it also returned `false` when Snowflake refused to renew the session for any other reason, for example because the account no longer allows the client's IP address. `isUp()` doesn't contact Snowflake. To check whether a connection can still run queries, use `await connection.isValidAsync()`, which sends a request to Snowflake and returns `false` in these cases too.
- Errors
  - Changed the errors that the driver reports for missing or invalid connection options to be an `InvalidParameterError` without a `code`. The old driver checked each option in its own way, so its errors were not consistent: most were an `InvalidParameterError` or a `MissingParameterError` with a code such as `404008`, but some were a plain `Error` with no code. The new driver checks all options in one shared step, so it can no longer give each problem its own code. Instead, the error `message` names every option that is missing or invalid. If your code checks `error.code` or `MissingParameterError`, check `error.name === 'InvalidParameterError'` instead.
  - Changed the error for a query that fails because its session expired or no longer exists. It is now an `OperationFailedError` with the Snowflake error code (`'390114'` or `'390111'`) and `sqlState` `'08001'`, instead of a `ClientError` with code `407002` and `sqlState` `'08003'`. `isFatal` is still `true`, and later queries on the same connection still fail with `407002`. If your code reconnects after a `407002` error, check `isFatal` instead.
- Removed undocumented APIs. They were never meant to be public and were not part of the TypeScript types:
  - The `snowflake.STRING`, `snowflake.BOOLEAN`, `snowflake.NUMBER`, `snowflake.DATE`, `snowflake.OBJECT`, `snowflake.ARRAY`, `snowflake.MAP` and `snowflake.JSON` constants. No public API used them.
  - The `connection.isTokenValid()` method.
  - The `getCurrentDatabaseProvider()` method on the object returned by `statement.getSessionState()`. It always returned `null`.
  - The `sessionToken`, `masterToken`, `sessionTokenExpirationTime` and `masterTokenExpirationTime` options of `snowflake.createConnection()`. They were not part of `ConnectionOptions`. To reuse a session, use `connection.serialize()` and `snowflake.deserializeConnection()`.

### New Features

- Added `isDecfloat()`, `isIntervalYearMonth()`, `isIntervalDayTime()` and `isVector()` to `Column`, for data types that had no such check.
- Added the `SnowflakeDate` export, the class of the values returned for `DATE` and `TIMESTAMP` columns. It extends `Date` and adds `getEpochSeconds()`, `getNanoSeconds()`, `getScale()`, `getTimezone()` and `getFormat()`. These methods existed before but had no types, so TypeScript code can now use them after a `value instanceof SnowflakeDate` check instead of a hand-written type.
- Added the `SessionState` type. `statement.getSessionState()` now returns `SessionState | undefined` instead of `object | undefined`, so TypeScript code can call `getCurrentDatabase()`, `getCurrentSchema()`, `getCurrentWarehouse()` and `getCurrentRole()` without a cast. Each method returns `string | null`.
- Added the `connectionName` connection option, which chooses a connection profile by name. You can pass other connection options next to it. The driver starts from the profile and replaces only the options you pass, for example to use a different `warehouse` than the profile sets. Without it, a profile is read only when `createConnection()` gets no options, as before.
- Added support for connection profiles in [`config.toml`](https://docs.snowflake.com/en/developer-guide/snowflake-cli/connecting/configure-cli), the configuration file that other Snowflake tools also use. Profiles go under `[connections.<name>]`, and the `default_connection_name` key sets the profile used when `createConnection()` gets no options.
- Added support for `workloadIdentityImpersonationPath` with the `AZURE` workload identity provider. The path must contain exactly one service principal client ID.
- Added support for a base64-encoded DER key in the `privateKey` connection option. Before, only a PEM string was accepted.
- Added the `protocol` and `port` options to `ConnectionOptions`. They set the scheme and port used with `host`, and now apply to every request, including external browser and Okta logins, which before always used `https` on the default port.
- Added storing cached login tokens in the system keyring, such as the macOS Keychain, Windows Credential Manager or a Linux keyring. When a keyring is available, the driver now uses it by default instead of a JSON file on disk. The JSON file is still used when no keyring is available.
- Added `sqlState` `'28000'` to errors for logins that Snowflake rejects because of wrong credentials, such as a wrong password or an invalid key-pair token.

### Changes

- Removed the client-side length check for the `queryTag` connection option. Snowflake still checks it when you connect, so an invalid `queryTag` can still fail when `connect()` or `connectAsync()` is called.
- Removed the format check for the `application` connection option. Any string is now accepted, because Snowflake does not check this value.
- Changed the format of the string returned by `connection.serialize()`. As documented, this format can change between driver versions, so pass the string to `snowflake.deserializeConnection()` on the same driver version only.
- Changed `connection.getQueryStatus()` and `connection.getQueryStatusThrowIfError()` to return `'NO_DATA'` instead of the undocumented `'NO_QUERY_DATA'` for a query ID that Snowflake doesn't know. `'NO_DATA'` counts as still running, so `connection.getResultsFromQueryId()` no longer fails right away for such an ID.

### Bugfixes

- Fixed `representNullAsStringNull` set on one connection also applying to every other connection in the same process.
- Fixed `TIME` columns returning the string `'NULL'` instead of `null` for NULL values. All other types already returned `null`.
- Fixed missing `status` argument in `connection.isAnError()` types. The method is now typed as `isAnError(status: QueryStatus): boolean`.
- Fixed `connection.getQueryStatus()` and `connection.getQueryStatusThrowIfError()` being typed to return `Promise<string>`, which meant TypeScript code had to cast the result before passing it to `isStillRunning()` or `isAnError()`. They are now typed as `getQueryStatus(queryId: string): Promise<QueryStatus>` and `getQueryStatusThrowIfError(queryId: string): Promise<QueryStatus>`.
- Fixed the `BLOCKED` query status being treated as a failure. A blocked query has not finished and can still succeed, so `isStillRunning('BLOCKED')` now returns `true` and `isAnError('BLOCKED')` returns `false`. `getQueryStatusThrowIfError()` no longer throws for it, and `getResultsFromQueryId()` waits for the query to finish instead of failing.
- Fixed `Column.isObject()` and `Column.isArray()` returning `false` for `OBJECT` and `ARRAY` columns, even though `getType()` returned `'object'` or `'array'`.
- Fixed `statement.getNumUpdatedRows()` returning `-1` for `COPY INTO <table>`. It now returns the number of rows loaded.
- Fixed `connection.getQueryStatusThrowIfError()` rejecting with code `-1` when the query failed on the server. The error now has the query's Snowflake error code, for example `'002003'`.
- Fixed `pool.acquire()` and `pool.use()` rejecting with a plain `Error` when a pooled connection fails to connect. They now reject with the original connect error, including its `name`, `code` and `sqlState`.
- Fixed the `Bind` type missing `Date`. The driver has always accepted `Date` values in `binds`, but the type did not list it.
- Fixed a `QUERY_TAG` passed in the `parameters` option of `connection.execute()` being replaced by the `queryTag` connection option. The statement's `QUERY_TAG` now applies to that query, and later queries still use the connection's `queryTag`.
- Fixed `ALTER SESSION SET CLIENT_STAGE_ARRAY_BINDING_THRESHOLD` having no effect when the `arrayBindingThreshold` connection option is set. The option now sets only the starting value of the session parameter, so the value set last decides when binds are uploaded to a stage.
- Fixed `FLOAT` columns returning `NaN` for the values `inf` and `-inf`. They now return `Infinity` and `-Infinity`, so they can be told apart from a real `NaN`.
- Fixed `snowflake.configure()` dropping an earlier `customLogger` or `additionalLogToConsole` value when a later call sets another logging option, such as `logLevel`. Logging options now keep their earlier values, like every other `configure()` option, and a call that sets only `additionalLogToConsole` now takes effect.
- Fixed `snowflake.deserializeConnection()` ignoring the `clientSessionKeepAlive` option.
- Fixed `PUT` failing with a `TypeError` when `SOURCE_COMPRESSION` is `BROTLI`, `AUTO_DETECT` or `NONE`.
- Fixed `sourceCompression` in `PUT` results always being `null`. It now shows the compression type of the source file, such as `'NONE'` or `'GZIP'`.
- Fixed `targetSize` in `PUT` results not matching the size of the file stored in the stage. On stages with client-side encryption, it was the size before encryption. It is now the size after encryption, which is a little larger.
- Fixed `PUT` failing when the source path starts with `~` or contains the `?` wildcard. `~` now means your home directory, and `?` matches any single character.

### Internal

- Changed how `ISA` is reported in telemetry to be consistent with other drivers.
- Changed when `PLATFORM` is reported in telemetry, so it no longer blocks the first connection request.
