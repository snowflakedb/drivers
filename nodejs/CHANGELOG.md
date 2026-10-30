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
- Changed `connection.connectAsync()` to report every error by rejecting the returned promise, and removed its callback parameter. Before, errors went to two different places: login errors, such as a wrong password, went only to the callback while the promise still resolved, and setup errors, such as an unsupported `authenticator`, rejected the promise. Handle all errors with `try`/`catch` around `await connection.connectAsync()` or with `.catch()`.
- Data Types
  - Changed integer columns to return a native JavaScript `BigInt` instead of a `big-integer` object when the `jsTreatIntegerAsBigInt` connection option or the `JS_TREAT_INTEGER_AS_BIGINT` session parameter is enabled. `BigInt` is built into JavaScript and needs no extra package. Both types hold the same values and work mostly the same way. The difference is that `big-integer` adds its own helper methods on top. If you use them, for example `.equals()`, `.add()` or `BigInteger.isInstance()`, switch to `BigInt` operators such as `===` and `+`, and to `typeof value === 'bigint'`.
  - Changed `BINARY` columns to return a plain `Buffer`. Before, the driver added two undocumented methods, `toStringSf()` and `getFormat()`, to every `Buffer` it returned. These methods were removed. To get the value as text, use `buffer.toString('hex').toUpperCase()` or `buffer.toString('base64')`.
  - Changed `VARIANT`, `OBJECT`, `ARRAY` and `MAP` values to be parsed with `JSON.parse()` only. Before, the driver evaluated values that were not valid JSON as JavaScript code. We removed this as a safety precaution and because `JSON.parse()` is much faster.
    By default (the `STRICT_JSON_OUTPUT` parameter is `FALSE`), Snowflake can return `NaN`, `Infinity` and `undefined` in these values, and they are not valid JSON. For example, `OBJECT_CONSTRUCT('a', 'NaN'::FLOAT)` returns `{"a": NaN}`, and `ARRAY_CONSTRUCT(1, NULL, 2)` returns `[1, undefined, 2]`. Queries that return such values now fail with an error. To read them, either:
    - set `STRICT_JSON_OUTPUT` to `TRUE`. Snowflake then returns `null` instead of `undefined`, and the strings `"NaN"` and `"Infinity"`.
    - pass your own `jsonColumnVariantParser` to `snowflake.configure()`.
  - Changed `VECTOR` columns to return an array of numbers, such as `[1.5, 2.5, 3.5]`, instead of the text sent by the server, such as `'[1.500000,2.500000,3.500000]'`. If your code parses that text, use the array directly. To get a string instead, add `'Number'` to `fetchAsString`: the array is then returned as JSON text, such as `'[1.5,2.5,3.5]'`, and a NULL value as `'NULL'`, the same as for other numbers.
- Removed undocumented APIs. They were never meant to be public and were not part of the TypeScript types:
  - The `snowflake.STRING`, `snowflake.BOOLEAN`, `snowflake.NUMBER`, `snowflake.DATE`, `snowflake.OBJECT`, `snowflake.ARRAY`, `snowflake.MAP` and `snowflake.JSON` constants. No public API used them.
  - The `connection.isTokenValid()` method.
  - The `getCurrentDatabaseProvider()` method on the object returned by `statement.getSessionState()`. It always returned `null`.
  - The `sessionToken`, `masterToken`, `sessionTokenExpirationTime` and `masterTokenExpirationTime` options of `snowflake.createConnection()`. They were not part of `ConnectionOptions`. To reuse a session, use `connection.serialize()` and `snowflake.deserializeConnection()`.
- Removed the `disableConsoleLogin` connection option. It is no longer needed because Snowflake already chooses the right login page. External browser login always opens the URL that Snowflake returns, as it did with the default value `true`:
  - If the account has the `ENABLE_IDENTIFIER_FIRST_LOGIN` parameter turned on, Snowflake returns its login page, where the user picks one of the account's identity providers.
  - Otherwise the account can have only one identity provider, and the browser goes straight to it.

### New Features

- Added utils for missing data types on a `Column` class: `isDecfloat()`, `isIntervalYearMonth()`, `isIntervalDayTime()` and `isVector()`.
- Added the `SnowflakeDate` export, the class of the values returned for `DATE` and `TIMESTAMP` columns. It extends `Date` and adds `getEpochSeconds()`, `getNanoSeconds()`, `getScale()`, `getTimezone()` and `getFormat()`. These methods existed before but had no types, so TypeScript code can now use them after a `value instanceof SnowflakeDate` check instead of a hand-written type.
- Added the `SessionState` type. `statement.getSessionState()` now returns `SessionState | undefined` instead of `object | undefined`, so TypeScript code can call `getCurrentDatabase()`, `getCurrentSchema()`, `getCurrentWarehouse()` and `getCurrentRole()` without a cast. Each method returns `string | null`.
- Added the `connectionName` connection option, which chooses a connection profile by name. You can pass other connection options next to it. The driver starts from the profile and replaces only the options you pass, for example to use a different `warehouse` than the profile sets. Without it, a profile is read only when `createConnection()` gets no options, as before.
- Added support for connection profiles in [`config.toml`](https://docs.snowflake.com/en/developer-guide/snowflake-cli/connecting/configure-cli), the configuration file that other Snowflake tools also use. Profiles go under `[connections.<name>]`, and the `default_connection_name` key sets the profile used when `createConnection()` gets no options.
- Added support for `workloadIdentityImpersonationPath` with the `AZURE` workload identity provider. The path must contain exactly one service principal client ID.

### Bugfixes

- Fixed `representNullAsStringNull` set on one connection also applying to every other connection in the same process.
- Fixed `TIME` columns returning the string `'NULL'` instead of `null` for NULL values. All other types already returned `null`.
- Fixed missing `status` argument in `connection.isAnError()` types. The method is now typed as `isAnError(status: QueryStatus): boolean`.
- Fixed `Column.isObject()` and `Column.isArray()` returning `false` for `OBJECT` and `ARRAY` columns, even though `getType()` returned `'object'` or `'array'`.
- Fixed `statement.getNumUpdatedRows()` returning `-1` for `COPY INTO <table>`. It now returns the number of rows loaded.
- Fixed `connection.getQueryStatusThrowIfError()` rejecting with code `-1` when the query failed on the server. The error now has the query's Snowflake error code, for example `'002003'`.
- Fixed `pool.acquire()` and `pool.use()` rejecting with a plain `Error` when a pooled connection fails to connect. They now reject with the original connect error, including its `name`, `code` and `sqlState`.
- Fixed a `QUERY_TAG` passed in the `parameters` option of `connection.execute()` being replaced by the `queryTag` connection option. The statement's `QUERY_TAG` now applies to that query, and later queries still use the connection's `queryTag`.
- Fixed `FLOAT` columns returning `NaN` for the values `inf` and `-inf`. They now return `Infinity` and `-Infinity`, so they can be told apart from a real `NaN`.

### Internal

- Removed the client-side length check for the `queryTag` connection option. Snowflake still checks it when you connect, so an invalid `queryTag` can still fail when `connect()` or `connectAsync()` is called.
- Removed the format check for the `application` connection option. Any string is now accepted, because Snowflake does not check this value.
- Changed how `ISA` is reported in telemetry to be consistent with other drivers.
- Changed when `PLATFORM` is reported in telemetry, so it no longer blocks the first connection request.
