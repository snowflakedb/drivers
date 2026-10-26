// The public API of the `snowflake-sdk` package: every value and type users can import.
// `index.ts` explains how it is exported.
export { SnowflakeDate } from './query-result/SnowflakeDate.js';
export {
  Connection,
  createConnection,
  serializeConnection,
  deserializeConnection,
} from './connection.js';
export { default as ErrorCode } from './constants/ErrorCode.js';
export { createPool } from './create-pool.js';
export { normalizeConnectionOptions } from './normalize-connection-options.js';
export { getFipsStatus } from './core/index.js';
export { updateGlobalConfig as configure } from './global-config.js';

export type { ConnectionCallback, StatementOption, FetchResultOptions } from './connection.js';
export type { ConnectionOptions } from './public-types/index.js';
export type { SnowflakeError } from './error.js';
export type { Bind, Binds, InsertBinds } from './query-result/binds.js';
export type { RowStatement } from './query-result/RowStatement.js';
export type {
  StatementCallback,
  StreamOptions,
  DataType,
  FetchRowsOptions,
  Column,
  RowMode,
  QueryStatus,
  SessionState,
  StatementStatus,
} from './query-result/types.js';
export type {
  CustomCredentialManager,
  CustomParser,
  XMlParserConfigOption,
  ConfigureOptions,
} from './global-config.js';
export type { LogLevel, SnowflakeLogger } from './logging/index.js';
export type { Pool, PoolOptions } from './create-pool.js';
export type { FipsStatus } from './core/index.js';
