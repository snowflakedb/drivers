# Changelog

## 4.0.0-beta.0

### New Architecture

Version 4 is a full rewrite. Most of the code is now written in Rust and compiled into a native `.node` addon, which makes the driver much faster. It also takes less disk space and has only two other npm dependencies. The public API is the same, apart from the breaking changes listed below. Since this is a new codebase, expect some small differences in behavior, including different error messages and codes. If your code checks error text, test it before you upgrade.

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

> We're still working on the areas below, so their behavior may change in later beta releases, before the official 4.x release.

- FIPS mode is still in progress. The native library can be built for FIPS, but the prebuilt addons we publish are not FIPS builds, so `snowflake.getFipsStatus()` returns `{ isFips: false }` for them. We're still deciding how FIPS mode will be set up in 4.x. The Node.js FIPS settings, such as the `--enable-fips` and `--force-fips` flags or `crypto.setFips()`, have no effect on the new driver, because it does its cryptography in its native library instead of the built-in `crypto` module.
- Errors are still a work in progress. Error classes, `code` values and messages may change, and some errors have no `code` yet. Avoid relying on the exact error text.
- TypeScript types and their JSDoc are not final. Some methods and options have short or missing docs, and some types may still change.
- Some behavior still differs from 3.x and is being reviewed, mainly in these areas:
  - Query cancellation and long-running queries
  - Reading large results
  - Timeouts and retries
  - External browser, Okta, OAuth and workload identity logins
  - Proxy settings
  - Token caching and custom credential managers
- Performance is still being tuned. The new driver is already faster than 3.x in most cases, and more improvements are planned.
- Some features have gaps in test coverage. We're reviewing these gaps and adding tests in upcoming beta releases.

### Breaking Changes

> [!WARNING]
> This list of breaking changes is not final. We may add or remove entries based on feedback from the beta.

#### General

- Changed the minimum supported Node.js version from 20 to 22, because Node.js 20 has reached end of life. Upgrade to Node.js 22 or later before you update the driver.
- Changed queries to fail when a `VARIANT`, `OBJECT`, `ARRAY` or `MAP` value contains `NaN`, `Infinity` or `undefined`, which are not valid JSON. Snowflake sends these values by default, when the `STRICT_JSON_OUTPUT` parameter is `FALSE`. For example, `OBJECT_CONSTRUCT('a', 'NaN'::FLOAT)` returns `{"a": NaN}`, and `ARRAY_CONSTRUCT(1, NULL, 2)` returns `[1, undefined, 2]`. The driver now parses these values with `JSON.parse()` only. Before, it ran values that were not valid JSON as JavaScript code. We removed this as a safety precaution and because `JSON.parse()` is much faster. To read such values, either:
  - set `STRICT_JSON_OUTPUT` to `TRUE`. Snowflake then returns `null` instead of `undefined`, and the strings `"NaN"` and `"Infinity"`.
  - pass your own `jsonColumnVariantParser` to `snowflake.configure()`.
- The driver no longer honors most Node.js runtime flags, environment variables and global settings of the Node.js `tls`, `https`, `dns` or `crypto` modules, because it doesn't use these modules. For example, flags such as `--use-openssl-ca`, `--openssl-config`, `--tls-min-v1.3`, `--tls-cipher-list` or `--dns-result-order`, and code such as `tls.DEFAULT_MIN_VERSION = 'TLSv1.3'` or `dns.setDefaultResultOrder('ipv4first')`, have no effect on it. Only two Node.js environment variables still work:
  - `NODE_TLS_REJECT_UNAUTHORIZED=0` turns off TLS certificate checks.
  - `NODE_EXTRA_CA_CERTS` adds the certificates from the given file to the list of trusted certificates.

#### Removed

- Removed OCSP checks. Snowflake drivers now check for revoked certificates with CRLs (certificate revocation lists), because OCSP often caused connection failures and many certificate authorities are dropping it. CRL checks are off by default, matching the OCSP default since version 3.4.0. To turn them on, set the `certRevocationCheckMode` connection option to `ENABLED` (in place of OCSP fail-closed mode) or `ADVISORY` (in place of fail-open mode). These APIs were removed:
  - The `disableOCSPChecks` and `ocspFailOpen` options of `snowflake.configure()`.
  - The `useConnectionConfigProxyForOCSP` connection option.
  - The `connection.setupOcspPrivateLink()` method.
  - The `snowflake.ocspModes` export and the `OcspModes` type.
- Data Types
  - Removed the undocumented `toStringSf()` and `getFormat()` methods from the `Buffer` returned for `BINARY` columns, which is now a plain `Buffer`. To get the value as text, use `buffer.toString('hex').toUpperCase()` or `buffer.toString('base64')`.
  - Removed the undocumented `BR` value of `SOURCE_COMPRESSION` in `PUT`. To upload Brotli-compressed files, use the documented `BROTLI` value instead.
- `ConnectionOptions`
  - Removed the `clientRequestMFAToken` connection option. MFA token caching is now controlled by `clientStoreTemporaryCredential`.
  - Removed the `disableConsoleLogin` connection option. It is no longer needed because Snowflake already chooses the right login page. External browser login always opens the URL that Snowflake returns, as it did with the default value `true`:
    - If the account has the `ENABLE_IDENTIFIER_FIRST_LOGIN` parameter turned on, Snowflake returns its login page, where the user picks one of the account's identity providers.
    - Otherwise the account can have only one identity provider, and the browser goes straight to it.
  - Removed the `region` connection option, which was deprecated. Set `host` to the full host name of your account, or set `accessUrl`, instead.
  - Removed the `clientConfigFile` connection option and the easy logging feature that it configured. The driver no longer reads `sf_client_config.json`, whether its path comes from this option or the `SF_CLIENT_CONFIG_FILE` environment variable, or the file is in the driver or home directory. If you used this file, the driver no longer writes those logs. To set the log level and log file, use the `logLevel` and `logFilePath` options of `snowflake.configure()`.
  - Removed the `oauthHttpAllowed` connection option. It was deprecated and meant only for tests. `oauthTokenRequestUrl` must still use `https://`, but `http://` is now accepted without any option when the host is `localhost`, `127.0.0.1` or `::1`, for example for a local mock server.
  - Removed the `serviceName` connection option. The service name is an internal session value that applications should never set.
  - Removed the `includeRetryReason` connection option. Retried requests always include the HTTP status code that caused the retry, as they did by default, because Snowflake uses it to find out why requests are retried.
  - Removed the `validateDefaultParameters` connection option. TypeScript already reports a misspelled option name when you compile, and the driver now always logs a warning for an option that it doesn't know and ignores.
  - Removed the `forceStageBindError` connection option. It made array binds fail on purpose and was meant only for the driver's own tests.
  - Removed the `gcsUseDownscopedCredential` connection option. Snowflake already uses downscoped credentials for `PUT` and `GET` on Google Cloud Storage by default, so setting the option had no effect.
- `snowflake.configure()` options
  - Removed the experimental `enableExperimentalMultipartUploads` option. `PUT` and `GET` now always transfer large files in parts on Amazon S3 and Azure stages (see New Features).
- Methods
  - Removed the `connection.getServiceName()` method. The service name is an internal session value that applications should never depend on.
  - Removed the `statement.getStatementId()` method, which was deprecated. It returned the same query ID as `statement.getQueryId()`, so call `getQueryId()` instead.
  - Removed the `statement.fetchRows()` method, which was deprecated. Use `statement.streamRows()` instead, which returns the same rows as a `Readable` stream. Read it with `for await (const row of statement.streamRows())`, or replace the `each` callback with the `'data'` event and the `end` callback with the `'end'` and `'error'` events.
- Types
  - Removed the `FileAndStageBindStatement` type and moved its `hasNext()` and `NextResult()` methods to `RowStatement`, which every query returns. These methods step through the results of a multi-statement query and have nothing to do with files or stages, so declaring them only on `FileAndStageBindStatement` was a mistake in the types. For a single statement, `hasNext()` returns `false`. If your code uses `FileAndStageBindStatement`, replace it with `RowStatement`.
  - Removed the options that only apply to `connection.execute()` from `connection.fetchResult()` and `connection.getResultsFromQueryId()`. Their options were typed as `{ queryId: string } & Partial<StatementOption>`, but most of these options, such as `binds`, `parameters`, `requestId` or `asyncExec`, had no effect. They are now typed as the new `FetchResultOptions` type, with only `queryId`, `complete`, `streamResult`, `rowMode` and `fetchAsString`, so if your TypeScript code passes other options, remove them. The `sqlText` option is gone too; if you need the SQL text, keep it in your own code.
  - Removed the `excludeGuid` and `queryId` fields from the `StatementOption` type. `connection.execute()` never used them, so setting them had no effect. If your TypeScript code sets them, remove them. To get the result of an earlier query by its ID, use `connection.getResultsFromQueryId({ queryId })`.
- Removed undocumented APIs. They were never meant to be public and were not part of the TypeScript types:
  - The `snowflake.STRING`, `snowflake.BOOLEAN`, `snowflake.NUMBER`, `snowflake.DATE`, `snowflake.OBJECT`, `snowflake.ARRAY`, `snowflake.MAP` and `snowflake.JSON` constants. No public API used them.
  - The `connection.isTokenValid()` method.
  - The `connection.heartbeat()` and `connection.heartbeatAsync()` methods. To check that a connection still works, use `await connection.isValidAsync()`, which sends the same heartbeat request. To keep a session alive, set the `clientSessionKeepAlive` connection option.
  - The `getFileMetadata()` method on the statement returned for a `PUT` or `GET` command. It returned the raw response that Snowflake sent for the command. To see what happened to each file, read the result rows, for example in the `complete` callback.
  - The `getCurrentDatabaseProvider()` method on the object returned by `statement.getSessionState()`. It always returned `null`.
  - The `sessionToken`, `masterToken`, `sessionTokenExpirationTime` and `masterTokenExpirationTime` options of `snowflake.createConnection()`. They were not part of `ConnectionOptions`. To reuse a session, use `connection.serialize()` and `snowflake.deserializeConnection()`.

#### Changed

- Data Types
  - Changed `TIMESTAMP_TZ`, `TIMESTAMP_LTZ` and `TIME` values with a zero UTC offset to show the offset as `Z` instead of `+0000`, `+00:00` or `+00` when they are returned as text, for example `2024-01-15 10:30:00.000 Z`. This now matches the text that Snowflake returns for `TO_CHAR` and `::VARCHAR`. For timestamps, this applies to `fetchAsString: ['Date']` and `toJSON()`, so also to `JSON.stringify()` of rows. `TIME` values are always returned as text, so for them it applies whenever `TIME_OUTPUT_FORMAT` contains `TZH`. `new Date()` reads both forms the same way. If your code checks the offset text with a regex or a strict format, also accept `Z`.
  - Changed integer columns to return a native JavaScript `BigInt` instead of a `big-integer` object when the `jsTreatIntegerAsBigInt` connection option or the `JS_TREAT_INTEGER_AS_BIGINT` session parameter is enabled. `BigInt` is built into JavaScript and needs no extra package. Both types hold the same values and work mostly the same way. The difference is that `big-integer` adds its own helper methods on top. If you use them, for example `.equals()`, `.add()` or `BigInteger.isInstance()`, switch to `BigInt` operators such as `===` and `+`, and to `typeof value === 'bigint'`.
  - Changed `VECTOR` columns to return an array of numbers, such as `[1.5, 2.5, 3.5]`, instead of the text sent by the server, such as `'[1.500000,2.500000,3.500000]'`. If your code parses that text, use the array directly. To get a string instead, add `'Number'` to `fetchAsString`: the array is then returned as JSON text, such as `'[1.5,2.5,3.5]'`, and a NULL value as `'NULL'`, the same as for other numbers.
- Connections
  - Changed login tokens to be cached by default. The `clientStoreTemporaryCredential` connection option now defaults to `true` and also controls MFA token caching, in place of the removed `clientRequestMFAToken` option. External browser, OAuth authorization code and MFA logins now reuse a cached token, so users are asked to log in less often. MFA tokens are cached only when your account allows it. Tokens are stored in the system keyring when one is available, such as the macOS Keychain, Windows Credential Manager or a Linux keyring. Without a keyring, for example on headless Linux or in CI, they are stored in a JSON file in the `SF_TEMPORARY_CREDENTIAL_CACHE_DIR` directory, or in `$XDG_CACHE_HOME/snowflake` or `~/.cache/snowflake` when it is not set. If none of these directories can be used, the driver logs a warning and logs in without caching. To turn caching off, set `clientStoreTemporaryCredential: false`.
  - Changed `snowflake.createConnection()` to no longer check the values of connection options. A missing or invalid option now fails `connection.connect()` or `connection.connectAsync()` instead, because connecting is the step that reads the connection profile and key files and logs in, so all errors about your settings come from one place. If you wrap `createConnection()` in `try`/`catch` to handle missing or invalid options, handle these errors when you connect instead: in the `connect()` callback, or with `try`/`catch` around `await connection.connectAsync()`.
  - Changed `connection.connectAsync()` to report every error by rejecting the returned promise, and removed its callback parameter. Before, the promise rejected only for some setup errors. Login errors, such as a wrong password, went only to the callback, and the promise still resolved. A callback passed to `connectAsync()` is now never called, so handle errors with `try`/`catch` around `await connection.connectAsync()` or with `.catch()`.
  - Changed the `resultPrefetch` connection option to be another name for the `CLIENT_PREFETCH_THREADS` session parameter, the number of result chunks that the driver downloads at the same time. Snowflake now checks the value, and without the option the session uses its own `CLIENT_PREFETCH_THREADS` value, which is 4 unless it is set on the user or account. Before, the driver checked the value itself and downloaded up to `resultPrefetch + 1` chunks at once, 3 by default.
  - Changed connecting to fail with an `InvalidParameterError` when `authenticator` is not a supported value, for example because of a typo. Before, the driver logged a warning and fell back to password login, so the typo could go unnoticed when a `password` was also set.
  - Changed connecting to fail with an `InvalidParameterError` when `workloadIdentityProvider`, `workloadIdentityAzureEntraIdResource`, `workloadIdentityImpersonationPath` or `workloadIdentityAwsUseOutboundToken` is set but `authenticator` is not `WORKLOAD_IDENTITY`. Before, the driver ignored these options and logged in with the other authenticator, which hid a mistake in the connection settings. Remove these options, or set `authenticator` to `WORKLOAD_IDENTITY`.
  - Changed `connection.isUp()` to return `false` only when the session has ended: the login expired, or Snowflake reports that the session is closed or no longer exists. Before, it also returned `false` when Snowflake refused to renew the session for any other reason, for example because the account no longer allows the client's IP address. `isUp()` doesn't contact Snowflake. To check whether a connection can still run queries, use `await connection.isValidAsync()`, which sends a request to Snowflake and returns `false` in these cases too.
- Query Execution
  - Changed `connection.execute()` to fail with a `ClientError` with code `407001` when it is called while the connection is still logging in. Before, the driver held such queries and ran them once the login finished. The new driver doesn't hold queries. Wait until the connection is established before you run queries, for example with `await connection.connectAsync()` or inside the `connect()` callback.
  - Changed the TypeScript types of `statement.getQueryId()`, `getRequestId()`, `getSqlText()`, `getNumRows()` and `getColumn()` to include `undefined`, because these methods return `undefined` in some cases, as they did before: for example while the query is running, after it fails, or for a column that doesn't exist. They are now typed as `getQueryId(): string | undefined`, `getRequestId(): string | undefined`, `getSqlText(): string | undefined`, `getNumRows(): number | undefined` and `getColumn(columnIdentifier: string | number): Column | undefined`, and their docs say when they return `undefined`. The returned values did not change, so where TypeScript now reports an error but your code knows the value is set, for example in the `complete` callback of a successful query, check for `undefined` or add `!`.
  - Changed the `complete` callback of `connection.execute()` to receive `undefined` as its `rows` argument when `asyncExec` is `true`. Before, the driver put a `{ queryId }` object in `rows` by mistake: `rows` is typed as an array of rows or `undefined`, and an async query has no rows yet. The query ID has always been available from `statement.getQueryId()`, so use that instead.
  - Changed `connection.execute()` with `describeOnly: true` to fail for a multi-statement query. Before, it succeeded but returned only one placeholder column named `multiple statement execution`, not the columns of your statements, because Snowflake doesn't describe the statements of a multi-statement query. To get column metadata, run `describeOnly` on each statement separately.
  - Changed `GET` to fail when no file in the stage matches the path or `PATTERN`. Before, it returned an empty result and downloaded nothing, which also hid a wrong path. If a `GET` in your code can match no files, catch the error, or run `LIST` with the same path and `PATTERN` first.
  - Changed `connection.getQueryStatus()` and `connection.getQueryStatusThrowIfError()` to return `'NO_DATA'` instead of the undocumented `'NO_QUERY_DATA'` when Snowflake has no status for the query ID. Snowflake returns no status both for a query that hasn't started yet and for a query ID that doesn't exist, so the driver can't tell these cases apart. `'NO_DATA'` counts as still running, so a loop that waits while `connection.isStillRunning(status)` is `true` keeps waiting for a query ID that doesn't exist. If your code can pass such an ID, limit how long it waits.
- Errors
  - Changed the errors that the driver reports for missing or invalid connection options to be an `InvalidParameterError` without a `code`. The old driver checked each option in its own way, so its errors were not consistent: most were an `InvalidParameterError` or a `MissingParameterError` with a code such as `404008`, but some were a plain `Error` with no code. The new driver checks all options in one shared step, so it can no longer give each problem its own code. Instead, the error `message` names every option that is missing or invalid. If your code checks `error.code` or `MissingParameterError`, check `error.name === 'InvalidParameterError'` instead.
- Changed the type of the `xmlParserConfig` option of `snowflake.configure()` to `X2jOptions` from `fast-xml-parser`. `attributesGroupName: null` no longer compiles in TypeScript. Use `attributesGroupName: false` instead, which works the same way.

### New Features

- Connections
  - Added support for connection profiles in [`config.toml`](https://docs.snowflake.com/en/developer-guide/snowflake-cli/connecting/configure-cli), the configuration file that other Snowflake tools also use. Profiles go under `[connections.<name>]`, and the `default_connection_name` key sets the profile used when `createConnection()` gets no options.
  - Added the `connectionName` connection option, which chooses a connection profile by name. You can pass other connection options next to it. The driver starts from the profile and replaces only the options you pass, for example to use a different `warehouse` than the profile sets. Without it, a profile is read only when `createConnection()` gets no options, as before.
  - Added the `proxy` connection option, which sets the proxy as one URL, for example `http://user:password@proxy.example.com:8080`. The scheme can be `http` or `https` and defaults to `http`. Percent-encode special characters in the user name or password, for example `@` as `%40`. When `proxyHost`, `proxyPort`, `proxyProtocol`, `proxyUser` or `proxyPassword` is also set, it replaces the matching part of the URL.
  - Added the `protocol` and `port` options to `ConnectionOptions`. They set the scheme and port used with `host`, and now apply to every request, including external browser and Okta logins, which before always used `https` on the default port.
  - Added support for a base64-encoded DER key in the `privateKey` connection option. Before, only a PEM string was accepted.
  - Added support for `workloadIdentityImpersonationPath` with the `AZURE` workload identity provider. The path must contain exactly one service principal client ID.
  - Added `sqlState` `'28000'` to errors for logins that Snowflake rejects because of wrong credentials, such as a wrong password or an invalid key-pair token.
- Data Types
  - Added `isDecfloat()`, `isIntervalYearMonth()`, `isIntervalDayTime()` and `isVector()` to `Column`, for data types that had no such check.
  - Added the `SnowflakeDate` export, the class of the values returned for `DATE` and `TIMESTAMP` columns. It extends `Date` and adds `getEpochSeconds()`, `getNanoSeconds()`, `getScale()`, `getTimezone()` and `getFormat()`. These methods existed before but had no types, so TypeScript code can now use them after a `value instanceof SnowflakeDate` check instead of a hand-written type.
- TypeScript types
  - Added the `SessionState` type. `statement.getSessionState()` now returns `SessionState | undefined` instead of `object | undefined`, so TypeScript code can call `getCurrentDatabase()`, `getCurrentSchema()`, `getCurrentWarehouse()` and `getCurrentRole()` without a cast. Each method returns `string | null`.
  - Added the `CustomCredentialManager` type. The `customCredentialManager` option of `snowflake.configure()` is now typed as `CustomCredentialManager` instead of `object`. It declares the `read`, `write` and `remove` methods that the driver already required, so TypeScript reports a manager without them when you compile, instead of `configure()` throwing at runtime.
- Added `snowflake.getFipsStatus()` and the `FipsStatus` type. The method is typed as `getFipsStatus(): FipsStatus`, where `FipsStatus` is `{ isFips: boolean }`, and tells whether the driver's native library is compiled for FIPS.
- Added support for all `fast-xml-parser` options in the `xmlParserConfig` option of `snowflake.configure()`. Before, the driver used only `ignoreAttributes`, `alwaysCreateTextNode`, `attributeNamePrefix` and `attributesGroupName`, and ignored any other option.
- Added multipart uploads to `PUT` and downloads in parts to `GET` on Amazon S3 and Azure stages. Large files now transfer faster, and when a request fails, the driver repeats only that part instead of the whole file. This is always on, so the experimental `enableExperimentalMultipartUploads` option of `snowflake.configure()` was removed (see Breaking Changes). How it works:
  - A file is split into parts when its size, after compression and encryption, is at least the limit that Snowflake sends for each `PUT` and `GET` command. This limit is 200 MiB today. Smaller files are still sent in one request.
  - Each part is 8 MiB. For very large files, the parts are bigger, so the file stays within the limit on the number of parts: 10,000 on S3 and 50,000 on Azure.
  - Parts are sent in parallel. The `PARALLEL` option of `PUT` and `GET` sets how many requests run at once for the whole command. A single file uses at most 16 at once. The driver reads the file one part at a time, so it never holds the whole file in memory.
  - When an upload part fails, for example because the stage credentials expired, the driver retries only that part. If an S3 upload still fails, the driver cancels it, which deletes the parts already sent.
  - On Google Cloud Storage, `PUT` sends each file in one request on purpose. On GCS, one request is much faster than an upload in parts.
  - In version 3.x, with `enableExperimentalMultipartUploads` turned on, uploads were split from 64 MiB, parts were sent one at a time, and Google Cloud Storage uploads were split into parts too.

### Changes

- Changed the format of the string returned by `connection.serialize()`. As documented, the format can change between driver versions, so pass the string to `snowflake.deserializeConnection()` on the same driver version only. If several processes share serialized connections, upgrade them all at the same time.
- Removed the client-side length check for the `queryTag` connection option. Snowflake still checks it when you connect, so an invalid `queryTag` can still fail when `connect()` or `connectAsync()` is called.
- Removed the format check for the `application` connection option. Any string is now accepted, because Snowflake does not check this value.

### Bugfixes

#### Fixes that may affect your code

> These fixes change a value, error or type that the driver returns. If your code works around the old behavior, update it.

- Fixed `fetchAsString: ['JSON']` returning only `VARIANT` columns as strings. It now covers `OBJECT`, `ARRAY` and `MAP` columns too. This includes `FILE` columns, and `GEOGRAPHY` and `GEOMETRY` columns in the default GeoJSON format, because their `getType()` is `'object'`. Their values are returned as the JSON text sent by Snowflake, such as `'{\n  "key": "value"\n}'`, and NULL values as `'NULL'` unless `representNullAsStringNull` is `false`. If your code reads these columns as objects, parse them with `JSON.parse()`.
- Fixed `TIME` columns returning the string `'NULL'` instead of `null` for NULL values. All other types already returned `null`. If your code checks a `TIME` value for `'NULL'`, check for `null` instead.
- Fixed `FLOAT` columns returning `NaN` for the values `inf` and `-inf`. They now return `Infinity` and `-Infinity`, so they can be told apart from a real `NaN`. If your code finds these values with `Number.isNaN()`, use `!Number.isFinite()` instead.
- Fixed the `BLOCKED` query status being treated as a failure. A blocked query has not finished and can still succeed, so `isStillRunning('BLOCKED')` now returns `true` and `isAnError('BLOCKED')` returns `false`. `getQueryStatusThrowIfError()` no longer throws for it, and `getResultsFromQueryId()` waits for the query to finish instead of failing. If your code handles `BLOCKED` as a failure, remove that handling.
- Fixed `statement.getNumUpdatedRows()` returning `-1` for `COPY INTO <table>`. It now returns the number of rows loaded. If your code checks for `-1` after `COPY INTO`, use the returned number instead.
- Fixed `Column.isObject()` and `Column.isArray()` returning `false` for `OBJECT` and `ARRAY` columns, even though `getType()` returned `'object'` or `'array'`. Code that branches on these methods now takes the `OBJECT` or `ARRAY` branch for these columns.
- Fixed `connection.getQueryStatusThrowIfError()` rejecting with code `-1` when the query failed on the server. The error now has the query's Snowflake error code, for example `'002003'`. If your code checks for code `-1`, check the Snowflake error code instead.
- Fixed `pool.acquire()` and `pool.use()` rejecting with a plain `Error` when a pooled connection fails to connect. They now reject with the original connect error, including its `name`, `code` and `sqlState`. If your code checks the message of that error, check its `name` or `code` instead.
- Fixed `targetSize` in `PUT` results not matching the size of the file stored in the stage. On stages with client-side encryption, it was the size before encryption. It is now the size after encryption, which is a little larger.
- Fixed `Column.getScale()` and `Column.getPrecision()` being typed to return `number`. They return `null` for columns that have no scale or precision, such as `TEXT` columns. They are now typed as `getScale(): number | null` and `getPrecision(): number | null`, so TypeScript may ask you to handle `null` where you use these values.

#### Other fixes

- Fixed `representNullAsStringNull` set on one connection also applying to every other connection in the same process.
- Fixed missing `status` argument in `connection.isAnError()` types. The method is now typed as `isAnError(status: QueryStatus): boolean`.
- Fixed `connection.getQueryStatus()` and `connection.getQueryStatusThrowIfError()` being typed to return `Promise<string>`, which meant TypeScript code had to cast the result before passing it to `isStillRunning()` or `isAnError()`. They are now typed as `getQueryStatus(queryId: string): Promise<QueryStatus>` and `getQueryStatusThrowIfError(queryId: string): Promise<QueryStatus>`.
- Fixed the `Bind` type missing `Date`. The driver has always accepted `Date` values in `binds`, but the type did not list it.
- Fixed a `QUERY_TAG` passed in the `parameters` option of `connection.execute()` being replaced by the `queryTag` connection option. The statement's `QUERY_TAG` now applies to that query, and later queries still use the connection's `queryTag`.
- Fixed `ALTER SESSION SET CLIENT_STAGE_ARRAY_BINDING_THRESHOLD` having no effect when the `arrayBindingThreshold` connection option is set. The option now sets only the starting value of the session parameter, so the value set last decides when binds are uploaded to a stage.
- Fixed the `options` argument of `snowflake.configure()` being typed as optional. A call without arguments always threw a `TypeError`. The method is now typed as `configure(options: ConfigureOptions): void`, so TypeScript reports this mistake when you compile.
- Fixed `snowflake.configure()` dropping an earlier `customLogger` or `additionalLogToConsole` value when a later call sets another logging option, such as `logLevel`. Logging options now keep their earlier values, like every other `configure()` option, and a call that sets only `additionalLogToConsole` now takes effect.
- Fixed `snowflake.deserializeConnection()` ignoring the `clientSessionKeepAlive` option.
- Fixed `PUT` failing with a `TypeError` when `SOURCE_COMPRESSION` is `BROTLI`, `AUTO_DETECT` or `NONE`.
- Fixed `sourceCompression` in `PUT` results always being `null`. It now shows the compression type of the source file, such as `'NONE'` or `'GZIP'`.
- Fixed `PUT` failing when the source path starts with `~` or contains the `?` wildcard. `~` now means your home directory, and `?` matches any single character.

### Internal

- Changed how `ISA` is reported in telemetry to be consistent with other drivers.
- Changed when `PLATFORM` is reported in telemetry, so it no longer blocks the first connection request.
