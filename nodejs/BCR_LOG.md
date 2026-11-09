This document outlines API behavior changes that should be reviewed or addressed in the new driver.

### Browser authentication coordination

Old Node (`lib/authentication/auth_coordinator.ts`) keeps one in-flight external-browser or authorization-code login per `host:username:authenticator`. Concurrent connects, including a pool acquire, share that flow's token or error and each still creates its own session. This runs with temporary-credential storage on or off.

Core has no such coordinator. It has a prompt lock (`sf_core/src/rest/snowflake/prompt_lock.rs`) for external browser, MFA and OAuth authorization code. The lock is taken only when temporary-credential storage is on (the default), `disable_parallel_user_prompt` is true (the default) and `username` is set. One connect opens the browser and stores the token; the others wait and then read the token from the cache. A successful login therefore opens one browser on both drivers.

They differ in these cases:

- **Temporary-credential storage off, or no `username`.** Core takes no lock. Each concurrent connect sends its own authenticator-request and opens its own browser. Old Node still shares one browser flow.
- **The first login fails or times out, storage on.** Core stores nothing, releases the lock, and each waiting connect opens its own browser. Old Node gives every waiting connect that same error, so the failed flow is not tried again.

Decide whether those cases should keep core's separate retry or share one token or error the way old Node does.

### API Argument Validation

In the new driver, we will remove most runtime argument validation and instead rely on TypeScript's static type checking. Previously, we had multiple layers of validation, which sometimes led to inconsistent error handling between methods. Omitting redundant runtime validation is standard practice in TypeScript codebases, as static type checks catch most usage errors during development.

### statement.fetchRows and statement.streamRows have wrong TypeScript typing

The old driver types both as `(options?: StreamOptions): Readable`, but this is inaccurate:

- `fetchRows` is a callback API, not a stream. `options`, `options.each`, and `options.end` are all required at runtime (it throws `ERR_STMT_FETCH_ROWS_MISSING_OPTIONS` / `_MISSING_EACH` / `_MISSING_END` otherwise). `each(row)` is invoked per row (returning `false` stops iteration) and `end(err, statement)` is invoked once at completion or on error. The returned value is not a consumable `Readable`.
- `StreamOptions.end` is typed as `number` (a row-range index), but `fetchRows` requires `end` to be a completion **function**. The same field means two different things across the two methods, so they cannot faithfully share `StreamOptions`.
- `StreamOptions.each` is never read by `streamRows`: its `RowStream` destructures only `start`, `end`, and `fetchAsString`. `each` was only present because both methods were typed with one shared interface.
- The new driver removes `fetchRows`, which the old driver marks as deprecated (BD#173), and removes `each` from `StreamOptions` since `streamRows` does not use it.

## Known bugs in both drivers

- Statement-level format parameters are ignored (`fetchAsString`, `toJSON`, `getFormat`). `ALTER SESSION` is honored.
  - `DATE_OUTPUT_FORMAT`
  - `TIME_OUTPUT_FORMAT`
  - `TIMESTAMP_NTZ_OUTPUT_FORMAT`
  - `TIMESTAMP_OUTPUT_FORMAT`
- Binary data type doesn't honor BINARY_OUTPUT_FORMAT when fetchAsString is used (should behave similar to timestamp output formats)
- Binding `"42.0"` to a DECFLOAT column returns `"42"`, dropping the trailing zero. The `decfloat.feature` "should select decfloat using parameter binding" scenario specifies `42.0` as the returned value, so both drivers deviate from the shared spec; the tests assert the observed `"42"` and cite this entry.
- `serialize()` on a connection that never connected, or that was destroyed, writes `{ tokenInfo: {} }`. That string has no session. `serialize()` should throw a plain error when there are no tokens. The new driver's `deserializeConnection` already rejects this payload (BD#71). The old driver builds a disconnected connection from it.

## Future Breaking Changes (BCRs)

These are potential improvements to consider after the UD release:

- Reevaluate the `jsTreatIntegerAsBigInt` parameter; consider either always converting all fixed numeric values to `BigInt`, or using `BigInt` only when the value exceeds the safe integer range (using `Number.isSafeInteger()`), and review approaches for handling floating-point numbers in a similar, consistent manner.
- `VARIANT`, `OBJECT` and `ARRAY` values are parsed with `JSON.parse()` (BD#140), and XML values with `fast-xml-parser`. Other drivers return the text and let the user decide how to parse it. Doing the same would remove the `jsonColumnVariantParser` / `xmlColumnVariantParser` / `xmlParserConfig` options and the `fast-xml-parser` dependency.
- `Column` has no `isFloat()` / `isReal()` method for the REAL Snowflake data type; `isNumber()` returns `true` for both FIXED and REAL, so a caller cannot distinguish them without `getType() === 'real'`. To be perfectly correct we should have a dedicated predicate for REAL, matching the pattern of every other `is*()` method.
- Parameter binding maps every value to a coarse set of logical bind types (`BOOLEAN`, `VARIANT`, `FIXED`, `REAL`, `TEXT`) and relies on the server to recast the stringified value into the column's final type. The Python and JDBC drivers instead bind dedicated types (`DATE`, `TIME`, `TIMESTAMP_LTZ`/`NTZ`/`TZ`, `BINARY`) with epoch-based numeric encodings, which is more efficient and avoids the server-side recast. Aligning Node.js with the temporal/binary bind types the other drivers use would be the consistent fix. See `nodejs/src/query-result/binds.ts`.
- A bound JS `Date` is sent as `TEXT` with `Date.toJSON()`, the same as the old driver. The server-side recast of that text goes through the session's `DATE_INPUT_FORMAT` / `TIMESTAMP_INPUT_FORMAT` / `TIME_INPUT_FORMAT`, so a session with a non-default format rejects or misreads the value, and `?::TIME` never accepts the full ISO string. Dedicated temporal bind types (epoch-based `TIMESTAMP_LTZ`/`NTZ`/`TZ`, `DATE`, `TIME`) would not depend on session formats.
- Default `rowMode` is `'object'`, which keys each row by column name and silently overwrites duplicates (last value wins). `'object_with_renamed_duplicated_columns'` already exists as an opt-in that keeps every column by renaming the 2nd+ occurrence (`NAME_2`, `NAME_3`, …, skipping names already taken). Consider making that the default instead of `'object'`, so joins and other queries with repeated names do not drop values.
- `QueryStatus` can be exported enum. `connection.isStillRunning(status)` and `connection.isAnError(status)` could take status as both string or enum (no BCR). `connection.getQueryStatus()` could also return this enum. It would match the python driver (BCR).
- `connection.isStillRunning(status)` and `connection.isAnError(status)` are instance methods that do not use connection state; classifying a status does not belong on a connection instance. Python defines them as `@staticmethod`s on `Connection`. A later BCR could move them to static methods (or package-level utils) to match.
- `getResultsFromQueryId` is an async method that also accepts a `complete` callback. The Promise settles when status polling finishes; `complete` is invoked later when `fetchResult` has collected rows. A later BCR should drop `complete`, settle the Promise when fetch is finished, and always treat `streamResult` as true so callers can `streamRows()` after await. `fetchResult` remains the callback-shaped counterpart.
- Make a decision whether statement getter (`getNumRows()`, `getColumn()`, `getColumns()`, etc...) should return undefined on in-progress/failed statement (old driver behavior) or throw the error. Many of these methods are typed NOT to return undefined in old driver (see BD#13).
- Snowflake returns HTTP 400 without a descriptive message for an invalid query ID or request ID, so the new driver checks that both are UUIDs before sending the request (BD#54). If Snowflake returned a clear message, the driver would not need these checks.
- `complete`'s third argument is inconsistent when no rows are collected: `streamResult: true` passes `undefined`, while `describeOnly: true` (without `streamResult`) passes `[]`. Both should be `undefined`.
- `RowStatement.getSessionState()` is a snapshot of warehouse, database, schema, and role from that statement's query response. There is no connection-level getter today for the live session identity the connection cache already keeps (the view Python, JDBC, and ODBC expose). A later `Connection.getSessionState()` that reads that cache would give callers current session names without keeping a statement around, and would not replace the statement snapshot.
- `snowflake.configure()` CRL options (`crlDownloadTimeout`, `crlDownloadMaxSize`, `crlCacheValidityTime`, `crlCacheDir`) stay global (BD#123), but connection options can set the same keys and override the `configure()` value, because both pass the same check of accepted keys. Decide whether to reject them as connection options. `configure()` values, `useEnvProxy` included, are read when a connection is created, so a later `configure()` call does not affect existing connections, unlike the old driver.
- Millisecond options (`timeout`, `browserActionTimeout`, `crlDownloadTimeout`, `crlCacheValidityTime`) are rounded down to whole seconds and a positive value under 1000 becomes 1 second (BD#59, BD#122, BD#123). Decide whether to keep milliseconds and document the rounding, or take seconds as other drivers do ([SNOW-4263660](https://snowflakecomputing.atlassian.net/browse/SNOW-4263660)).
- JWT `ConnectionOptions.privateKey` accepts only strings. Consider widening it to `string | Buffer` so Node can implement the shared **should authenticate using private_key as bytes** scenario. For a Buffer containing unencrypted PKCS8 DER, base64-encode it in the Node option normalizer before the existing string-only core boundary; `sf_core` already accepts base64 DER. Do not use `String(buffer)`, which corrupts binary key material.
- `CLIENT_STAGE_ARRAY_BINDING_THRESHOLD` passed in the `parameters` option of `connection.execute()` should apply to that statement only and win over the session value, whether that value comes from the `arrayBindingThreshold` connection option or from `ALTER SESSION` (BD#26). Both drivers ignore it today. The server does not use this parameter, so the driver has to read it from the statement parameters itself before it chooses between stage and inline binds.
- `proxy` (the proxy URL) is merged with `proxyHost` / `proxyPort` / `proxyProtocol` / `proxyUser` / `proxyPassword`: each discrete field overrides the matching URL component. However this behavior is inconsistent with `accessUrl`. When it is set, `host` / `port` / `protocol` are not applied to the login URL. This should be unified.
