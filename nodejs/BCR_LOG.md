This document outlines API behavior changes that should be reviewed or addressed in the new driver.

### Browser authentication coordination

TODO: To be verified.

Old Node (`lib/authentication/auth_coordinator.ts`) keeps one in-flight external-browser or authorization-code login per `host:username:authenticator`. Concurrent connects, including a pool acquire, share that flow's token or error and each still creates its own session. This runs with temporary-credential storage on or off.

Core's prompt lock runs when temporary-credential storage is on (the default) and `disable_parallel_user_prompt` is true (the default). One connect opens the browser and stores the token; the others wait and reuse it. A successful login therefore opens one browser on both drivers.

They differ in two cases:

- **Temporary-credential storage off.** Core takes no lock. Each concurrent connect sends its own authenticator-request and opens its own browser. Old Node still shares one browser flow.
- **The first login fails or times out, storage on.** Core stores nothing, releases the lock, and each waiting connect opens its own browser. Old Node gives every waiting connect that same error, so the failed flow is not tried again.

Decide whether those two cases should keep core's separate retry or share one token or error the way old Node does.

### Azure WIF client-id end-to-end coverage

`workloadIdentityAzureClientId` maps to `workload_identity_azure_client_id` and
selects the user-assigned Azure managed identity used for WIF attestation. Local
sf_core tests verify that it becomes the IMDS `client_id` parameter and overrides
`MANAGED_IDENTITY_CLIENT_ID`, but neither the Node.js nor sf_core live WIF suite
sets it.

[SNOW-4232648](https://snowflakecomputing.atlassian.net/browse/SNOW-4232648)
tracks attaching a user-assigned identity to the Azure WIF test VM, mapping it
to a dedicated Snowflake user, and adding live sf_core and Node.js tests that
assert that user. The test should require the preconfigured identity rather
than create Azure resources during a test run.

### API Argument Validation

In the new driver, we will remove most runtime argument validation and instead rely on TypeScript's static type checking. Previously, we had multiple layers of validation, which sometimes led to inconsistent error handling between methods. Omitting redundant runtime validation is standard practice in TypeScript codebases, as static type checks catch most usage errors during development.

### RowStatement

The following methods are wrong because the old driver actually returns `undefined` if called before query completion or when query returns error/no rows:

- `getNumRows(): number;`
- `getQueryId(): string;`

### statement.fetchRows and statement.streamRows have wrong TypeScript typing

The old driver types both as `(options?: StreamOptions): Readable`, but this is inaccurate:

- `fetchRows` is a callback API, not a stream. `options`, `options.each`, and `options.end` are all required at runtime (it throws `ERR_STMT_FETCH_ROWS_MISSING_OPTIONS` / `_MISSING_EACH` / `_MISSING_END` otherwise). `each(row)` is invoked per row (returning `false` stops iteration) and `end(err, statement)` is invoked once at completion or on error. The returned value is not a consumable `Readable`.
- `StreamOptions.end` is typed as `number` (a row-range index), but `fetchRows` requires `end` to be a completion **function**. The same field means two different things across the two methods, so they cannot faithfully share `StreamOptions`.
- `StreamOptions.each` is never read by `streamRows`: its `RowStream` destructures only `start`, `end`, and `fetchAsString`. `each` was only present because both methods were typed with one shared interface.
- In the new driver, `fetchRows` gets a dedicated `FetchRowsOptions` (required `each` / `end` callbacks, no misleading `Readable` return), and `each` is removed from `StreamOptions` since `streamRows` does not use it.

### connection.fetchResult and connection.getResultsFromQueryId has wrong TypeScript typing

- they both require `queryId` which is optional in StatementOption
- most of `StatementOption` does not apply to fetch/get results. They work only for `.execute()`
- `getResultsFromQueryId` has inconsistent async patterns: it's an async method but also accepts a `complete` callback. The Promise settles when status polling finishes; `complete` is invoked later when `fetchResult` has collected rows. Taking both a callback and a Promise is a poor Node API. A later BCR should drop `complete`, settle the Promise when fetch is finished, and always treat `streamResult` as true so callers can `streamRows()` / `fetchRows()` after await. `fetchResult` remains the callback-shaped counterpart.

### snowflake .connectAsync(callback)

- In the old driver, this method, in some cases, returns errors via `callback(err)` and in other cases throws errors directly. This inconsistent behavior is a bug. In the new driver, async methods should not accept a callback, and all errors should be handled via rejected promises.

### connection.heartbeat(callback) and .heartbeatAsync()

- These methods are publicly exported but not documented. There is no practical use case for end users, as heartbeat is sent automatically by the driver.

### statement.getColumn() API

- The `isArray` and `isObject` methods are bugged and return false because server doesn't return `fieldsMetadata`

### TIME has no month, but date elements in a TIME format render the Unix epoch

TIME is a clock (`14:45:30`), not a date, yet `YYYY-MM-DD` against a TIME renders
`1970-01-01` and `MMMM` renders `January`. Snowflake's `TO_CHAR` treats a TIME as
seconds-since-midnight on an instant, which at UTC is the Unix epoch — not a quirk
of the old Node format converter.

## Known bugs in both drivers

- Statement-level format parameters are ignored (`fetchAsString`, `toJSON`, `getFormat`). `ALTER SESSION` is honored.
  - `DATE_OUTPUT_FORMAT`
  - `TIME_OUTPUT_FORMAT`
  - `TIMESTAMP_NTZ_OUTPUT_FORMAT`
  - `TIMESTAMP_OUTPUT_FORMAT`
- Binary data type doesn't honor BINARY_OUTPUT_FORMAT when fetchAsString is used (should behave similar to timestamp output formats)
- Binding `"42.0"` to a DECFLOAT column returns `"42"`, dropping the trailing zero. The `decfloat.feature` "should select decfloat using parameter binding" scenario specifies `42.0` as the returned value, so both drivers deviate from the shared spec; the tests assert the observed `"42"` and cite this entry.
- `fetchAsString: ['JSON']` stringifies only `variant`. `object`, `array`, and `map` have no string converter, so the parsed JS value is left in place. FILE, GEOMETRY, and GEOGRAPHY report as `object` and inherit that gap. They should stringify like VARIANT.
- `serialize()` on a connection that never connected, or that was destroyed, writes `{ tokenInfo: {} }`. That string has no session. `serialize()` should throw a plain error when there are no tokens. The new driver's `deserializeConnection` already rejects this payload (BD#71). The old driver builds a disconnected connection from it.

## Future Breaking Changes (BCRs)

These are potential improvements to consider after the UD release:

- `snowflake.deserializeConnection` should throw an exception when provided an invalid or malformed serialized string, instead of failing in some cases and returning a disconnected connection.
- Reevaluate the `jsTreatIntegerAsBigInt` parameter; consider either always converting all fixed numeric values to `BigInt`, or using `BigInt` only when the value exceeds the safe integer range (using `Number.isSafeInteger()`), and review approaches for handling floating-point numbers in a similar, consistent manner.
- Variant JSON/XML parsing is a mess: it is slow, does eval() and adds 6 dependencies (2MB). We should follow other drivers and let user decide how to parse variants. See "parses JSON with undefined, Infinity, NaN as JS types" test
- `Column` has no `isFloat()` / `isReal()` method for the REAL Snowflake data type; `isNumber()` returns `true` for both FIXED and REAL, so a caller cannot distinguish them without `getType() === 'real'`. To be perfectly correct we should have a dedicated predicate for REAL, matching the pattern of every other `is*()` method.
- Parameter binding maps every value to a coarse set of logical bind types (`BOOLEAN`, `VARIANT`, `FIXED`, `REAL`, `TEXT`) and relies on the server to recast the stringified value into the column's final type. The Python and JDBC drivers instead bind dedicated types (`DATE`, `TIME`, `TIMESTAMP_LTZ`/`NTZ`/`TZ`, `BINARY`) with epoch-based numeric encodings, which is more efficient and avoids the server-side recast. Aligning Node.js with the temporal/binary bind types the other drivers use would be the consistent fix. See `nodejs/src/query-result/binds.ts`.
- Binding a JS `Date` is not supported. The old driver's `Bind` type (`string | number | boolean | null`) never included `Date`, but its runtime accepted one passed past the type check and bound it as `TEXT` via `Date.toJSON()`. The new driver has no `Date` handling: it binds as `VARIANT`, so casts such as `?::TIMESTAMP_NTZ` fail (BD#86). The server-side recast of a text date goes through the session's `DATE_INPUT_FORMAT` / `TIMESTAMP_INPUT_FORMAT` / `TIME_INPUT_FORMAT`, so a session with a non-default format rejects or misreads the value, and `?::TIME` never accepts the full ISO string. `Date` support should come back only with dedicated temporal bind types (epoch-based `TIMESTAMP_LTZ`/`NTZ`/`TZ`, `DATE`, `TIME`) that do not depend on session formats.
- Default `rowMode` is `'object'`, which keys each row by column name and silently overwrites duplicates (last value wins). `'object_with_renamed_duplicated_columns'` already exists as an opt-in that keeps every column by renaming the 2nd+ occurrence (`NAME_2`, `NAME_3`, …, skipping names already taken). Consider making that the default instead of `'object'`, so joins and other queries with repeated names do not drop values.
- `QueryStatus` can be exported enum. `connection.isStillRunning(status)` and `connection.isAnError(status)` could take status as both string or enum (no BCR). `connection.getQueryStatus()` could also return this enum. It would match the python driver (BCR).
- `connection.isStillRunning(status)` and `connection.isAnError(status)` are instance methods that do not use connection state; classifying a status does not belong on a connection instance. Python defines them as `@staticmethod`s on `Connection`. A later BCR could move them to static methods (or package-level utils) to match.
- `getResultsFromQueryId` should drop `complete`, settle the returned Promise when fetch is finished, and always treat `streamResult` as true. Taking a callback and returning a Promise is a mixed pattern; see the typing section above.
- Make a decision whether statement getter (`getNumRows()`, `getColumn()`, `getColumns()`, etc...) should return undefined on in-progress/failed statement (old driver behavior) or throw the error. Many of these methods are typed NOT to return undefined in old driver (see BD#13).
- Snowflake server can return status 400 for invalid parameters but it does not return descriptive messages. Snowflake server should fix it and drivers should not validate query_id or request_id themselves.
- `complete`'s third argument is inconsistent when no rows are collected: `streamResult: true` passes `undefined`, while `describeOnly: true` (without `streamResult`) passes `[]`. Both should be `undefined`.
- `describeOnly` doesn't work for multistatements in snowflake server. The Server does not return child queries ids in the response if `describeOnly` is present so it's not possbile to fetch results.
- `RowStatement.getSessionState()` is a snapshot of warehouse, database, schema, and role from that statement's query response. There is no connection-level getter today for the live session identity the connection cache already keeps (the view Python, JDBC, and ODBC expose). A later `Connection.getSessionState()` that reads that cache would give callers current session names without keeping a statement around, and would not replace the statement snapshot.
- JWT `ConnectionOptions.privateKey` accepts only strings. Consider widening it to `string | Buffer` so Node can implement the shared **should authenticate using private_key as bytes** scenario. For a Buffer containing unencrypted PKCS8 DER, base64-encode it in the Node option normalizer before the existing string-only core boundary; `sf_core` already accepts base64 DER. Do not use `String(buffer)`, which corrupts binary key material.
