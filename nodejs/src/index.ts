// Entry point of the `snowflake-sdk` package.
//
// Code written for the old driver must keep working. It uses the package like this:
//
//   import snowflake from 'snowflake-sdk'; // or: const snowflake = require('snowflake-sdk');
//   const options: snowflake.ConnectionOptions = { account: 'myaccount' };
//   snowflake.createConnection(options);
//   vi.spyOn(snowflake, 'createConnection'); // in tests
//
// `require` and the default import must return the same object.
// Otherwise a stub set through one is not seen through the other.
//
// Named imports also work, for values and types:
// `import { createConnection, type ConnectionOptions } from 'snowflake-sdk'`.
//
// Why `api.ts` exists:
// - It lists the public API in one place, with plain `export ... from` lines.
//   Each module can export in its own style, default or named.
// - The `export import` lines below need a name inside a namespace, like `api.X`:
//   - A function or class imported by name (`import { createConnection }`) is rejected,
//     because it is not a namespace.
//   - A whole-module import (`import * as connection`) works, but not for a default export:
//     `connection.default` is rejected, because `default` is a reserved word there.
//   - With one `api` module, names are not repeated:
//     `api.normalizeConnectionOptions`, not `normalize.normalizeConnectionOptions`.
//
// To add a public export:
// - Add it to `api.ts`.
// - Function, class, or enum: add an `export import` line below.
// - Class, enum, interface, or type alias: add a `type` line to `declare namespace snowflake`
//   at the end of the file.
import * as api from './api.js';
import { coreInitBridge, coreLogEvent, coreSetLogLevel } from './core/index.js';
import { logRouter } from './logging/index.js';

const initBridgeResult = coreInitBridge(
  { SnowflakeDate: api.SnowflakeDate },
  logRouter.handleCoreLogEvent,
);
logRouter.setup({
  logEvent: coreLogEvent,
  setLogLevel: coreSetLogLevel,
  troubleshooting: initBridgeResult.troubleshooting,
});

// Values are exported with `export import`, because only this form gives both:
// - Users can replace an export, for example stub `createConnection` in tests.
//   `export { createConnection } from` compiles to a read-only getter, so it can not be replaced.
// - A class name works as a value and as a type: `new Connection()` and `conn: Connection`.
//   With `export const Connection = api.Connection`, it works only as a value.
export import SnowflakeDate = api.SnowflakeDate;
export import Connection = api.Connection;
export import ErrorCode = api.ErrorCode;
export import createConnection = api.createConnection;
export import createPool = api.createPool;
export import serializeConnection = api.serializeConnection;
export import deserializeConnection = api.deserializeConnection;
export import normalizeConnectionOptions = api.normalizeConnectionOptions;
export import getFipsStatus = api.getFipsStatus;
export import configure = api.configure;

// Exports every type from `api.ts` by name: `import type { ConnectionOptions } from 'snowflake-sdk'`.
// - Some names, like `Connection`, are also exported above with `export import`.
// - That export wins, so `Connection` stays a value too, not only a type.
export type * from './api.js';

// The default export is this module's own exports object (`self`):
// - The compiled file marks itself with `__esModule`, so TypeScript output and bundlers read
//   `.default` for `import snowflake from 'snowflake-sdk'`.
// - `.default` must be the same object as `module.exports`, so a stub set in one place is
//   visible everywhere. `export default api` does not work for this: `api` is another object,
//   and its exports are read-only getters.
//
// Why the namespace is needed:
// - TypeScript rejects `self` as the default export directly (circular import alias),
//   so the default export goes through a `const`.
// - A `const` holds only values. Without the namespace, `snowflake.ConnectionOptions` after
//   a default import is a type error. The old driver supports it, so user code relies on it.
// - A `declare namespace` with the same name merges with the `const` and adds the types.
//   It adds no code to the compiled output.
// tests/e2e/package-shape.test.ts checks each type here against its named export.
import * as self from './index.js';
const snowflake = self;
declare namespace snowflake {
  type SnowflakeDate = api.SnowflakeDate;
  type Connection = api.Connection;
  type ErrorCode = api.ErrorCode;
  type ConnectionOptions = api.ConnectionOptions;
  type ConnectionCallback = api.ConnectionCallback;
  type StatementOption = api.StatementOption;
  type FetchResultOptions = api.FetchResultOptions;
  type SnowflakeError = api.SnowflakeError;
  type Bind = api.Bind;
  type Binds = api.Binds;
  type InsertBinds = api.InsertBinds;
  type RowStatement = api.RowStatement;
  type StatementCallback = api.StatementCallback;
  type StreamOptions = api.StreamOptions;
  type DataType = api.DataType;
  type FetchRowsOptions = api.FetchRowsOptions;
  type Column = api.Column;
  type RowMode = api.RowMode;
  type QueryStatus = api.QueryStatus;
  type SessionState = api.SessionState;
  type StatementStatus = api.StatementStatus;
  type CustomCredentialManager = api.CustomCredentialManager;
  type CustomParser = api.CustomParser;
  type XMlParserConfigOption = api.XMlParserConfigOption;
  type ConfigureOptions = api.ConfigureOptions;
  type LogLevel = api.LogLevel;
  type SnowflakeLogger = api.SnowflakeLogger;
  type Pool<T> = api.Pool<T>;
  type PoolOptions = api.PoolOptions;
  type FipsStatus = api.FipsStatus;
}
export default snowflake;
