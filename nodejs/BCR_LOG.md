This document outlines API behavior changes that should be reviewed or addressed in the new driver.

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
- `getResultsFromQueryId` has inconsistent async patterns: it's an async method but also accepts a callback for error/result handling. This mixed pattern should be reviewed when designing the new async-first API for the entire driver

### snowflake .connectAsync(callback)

- In the old driver, this method, in some cases, returns errors via `callback(err)` and in other cases throws errors directly. This inconsistent behavior is a bug. In the new driver, async methods should not accept a callback, and all errors should be handled via rejected promises.

### connection.heartbeat(callback) and .heartbeatAsync()

- These methods are publicly exported but not documented. There is no practical use case for end users, as heartbeat is sent automatically by the driver.

### statement.getColumn() API

- The methods `getRowValue(row: object)` and `getRowValueAsString(row: object)` are publicly documented in `index.d.ts`, but they were never covered by tests and do not work as intended. The `(row: object)` parameter requires a special internal row class that is not exposed to users. The public API returns rows as `externalizeRow`, so calling these methods will result in a runtime error.
- The `is*` methods (e.g. `isString()`) do not cover every data type value that can be returned by `.getType()`. For example, `decfloat` is not covered by any `is*` method. The new driver adds `isDecfloat()` to close this specific gap; old driver has no equivalent method.
- The `isArray` and `isObject` methods are bugged and return false because server doesn't return `fieldsMetadata`

### Statement Buffer Monkey Patching

- Currently, when a query returns a BINARY column, the driver returns a Buffer object monkey-patched methods: `.toStringSf()` and `.getFormat()`. These methods are not part of the documented API. There is no use case for them, as node can convert Buffer to both hex and base64.

## Future Breaking Changes (BCRs)

These are potential improvements to consider after the UD release:

- `snowflake.serializeConnection` should throw or return null when called on a disconnected connection, rather than returning an unusable object.
- `snowflake.deserializeConnection` should throw an exception when provided an invalid or malformed serialized string, instead of failing in some cases and returning a disconnected connection.
- Reevaluate the `jsTreatIntegerAsBigInt` parameter; consider either always converting all fixed numeric values to `BigInt`, or using `BigInt` only when the value exceeds the safe integer range (using `Number.isSafeInteger()`), and review approaches for handling floating-point numbers in a similar, consistent manner.
- Variant JSON/XML parsing is a mess: it is slow, does eval() and adds 6 dependencies (2MB). We should follow other drivers and let user decide how to parse variants. See "parses JSON with undefined, Infinity, NaN as JS types" test
- `Column` has no `isFloat()` / `isReal()` method for the REAL Snowflake data type; `isNumber()` returns `true` for both FIXED and REAL, so a caller cannot distinguish them without `getType() === 'real'`. To be perfectly correct we should have a dedicated predicate for REAL, matching the pattern of every other `is*()` method.
- Default `rowMode` is `'object'`, which keys each row by column name and silently overwrites duplicates (last value wins). `'object_with_renamed_duplicated_columns'` already exists as an opt-in that keeps every column by renaming the 2nd+ occurrence (`NAME_2`, `NAME_3`, …, skipping names already taken). Consider making that the default instead of `'object'`, so joins and other queries with repeated names do not drop values.
