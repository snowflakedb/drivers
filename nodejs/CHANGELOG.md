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

> These issues will be fixed in upcoming beta releases, before the official 4.x release.

- FIPS mode is not supported yet. The Node.js FIPS settings, such as the `--enable-fips` and `--force-fips` flags or `crypto.setFips()`, only change how the built-in `crypto` module works. The new driver does its cryptography in its native library instead of that module, so these settings have no effect on it.

### Breaking Changes

> [!WARNING]
> This list of breaking changes is not final. We may add or remove entries based on feedback from the beta.

- Changed the minimum supported Node.js version from 20 to 22, because Node.js 20 has reached end of life. Upgrade to Node.js 22 or later before you update the driver.
- Data Types
  - Changed integer columns to return a native JavaScript `BigInt` instead of a `big-integer` object when the `jsTreatIntegerAsBigInt` connection option or the `JS_TREAT_INTEGER_AS_BIGINT` session parameter is enabled. `BigInt` is built into JavaScript and needs no extra package. If you use `big-integer` methods such as `.equals()` or `BigInteger.isInstance()`, switch to `BigInt` operators and `typeof value === 'bigint'`.
  - Changed `BINARY` columns to return a plain `Buffer`. Before, the driver added two undocumented methods, `toStringSf()` and `getFormat()`, to every `Buffer` it returned. These methods were removed. To get the value as text, use `buffer.toString('hex').toUpperCase()` or `buffer.toString('base64')`.

### New Features

- Added utils for missing data types on a `Column` class: `isDecfloat()`, `isIntervalYearMonth()` and `isIntervalDayTime()`
- Added the `SnowflakeDate` export, the class of the values returned for `DATE` and `TIMESTAMP` columns. It extends `Date` and adds `getEpochSeconds()`, `getNanoSeconds()`, `getScale()`, `getTimezone()` and `getFormat()`. These methods existed before but had no types, so TypeScript code can now use them after a `value instanceof SnowflakeDate` check instead of a hand-written type.

### Bugfixes

- Fixed `representNullAsStringNull` set on one connection also applying to every other connection in the same process.
- Fixed missing `status` argument in `connection.isAnError()` types. The method is now typed as `isAnError(status: QueryStatus): boolean`.

### Internal

- Removed the undocumented `snowflake.STRING`, `snowflake.BOOLEAN`, `snowflake.NUMBER`, `snowflake.DATE`, `snowflake.OBJECT`, `snowflake.ARRAY`, `snowflake.MAP` and `snowflake.JSON` constants. They were dead code: no public API used them.
- Removed the client-side length check for the `queryTag` connection option. Snowflake still checks it when you connect, so an invalid `queryTag` can still fail when `connect()` or `connectAsync()` is called.
- Removed the format check for the `application` connection option. Any string is now accepted, because Snowflake does not check this value.
