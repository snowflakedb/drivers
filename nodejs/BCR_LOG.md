This document outlines APIs that we should consider _not_ porting to the new driver.

### snowflake .connectAsync(callback)

- In the old driver, this method, in some cases, returns errors via `callback(err)` and in other cases throws errors directly. This inconsistent behavior is a bug. In the new driver, async methods should not accept a callback, and all errors should be handled via rejected promises.

### snowflake.STRING, BOOLEAN, NUMBER, DATE, OBJECT, ARRAY, MAP, JSON constants

- These constants are exported by the old driver but are undocumented. There does not appear to be a real use case for them, as the values returned by `column.getType()` do not match these constants.
- They were temporarily added to `snowflake-sdk-fixed` for compatibility, but should likely be removed.

### statement.getColumn() API

- The methods `getRowValue(row: object)` and `getRowValueAsString(row: object)` are publicly documented in `index.d.ts`, but they were never covered by tests and do not work as intended. The `(row: object)` parameter requires a special internal row class that is not exposed to users. The public API returns rows as `externalizeRow`, so calling these methods will result in a runtime error.
- The `is*` methods (e.g. `isString()`) do not cover every data type value that can be returned by `.getType()`. For example, `decfloat` is not covered by any `is*` method. Consider either extending these methods to handle all possible types, dropping them, or replacing them with a single type discriminator (such as an enum).
- The `isArray` and `isObject` methods are bugged and return false because server doesn't return `fieldsMetadata`

### Statement Buffer Monkey Patching

- Currently, when a query returns a BINARY column, the driver returns a Buffer object monkey-patched methods: `.toStringSf()` and `.getFormat()`. These methods are not part of the documented API.
