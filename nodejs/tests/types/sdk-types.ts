import * as newSnowflakeSdk from 'snowflake-sdk';
import oldSnowflakeSdk from 'snowflake-sdk-old';

export type NewSnowflakeSdkColumn = newSnowflakeSdk.Column;

export type SnowflakeError = newSnowflakeSdk.SnowflakeError | oldSnowflakeSdk.SnowflakeError;

export type Connection = newSnowflakeSdk.Connection | oldSnowflakeSdk.Connection;
export type ConnectionOptions =
  | newSnowflakeSdk.ConnectionOptions
  | oldSnowflakeSdk.ConnectionOptions;

export type RowStatement = newSnowflakeSdk.RowStatement | oldSnowflakeSdk.RowStatement;
export type StatementOption = newSnowflakeSdk.StatementOption | oldSnowflakeSdk.StatementOption;
export type Binds = newSnowflakeSdk.Binds | oldSnowflakeSdk.Binds;
export type RowMode = newSnowflakeSdk.RowMode | oldSnowflakeSdk.RowMode;
export type QueryStatus = newSnowflakeSdk.QueryStatus | oldSnowflakeSdk.QueryStatus;

// TODO: import SnowflakeDate from snowflake-sdk once the new driver exports it.
// The old driver does not export this type; the new driver should.
export type SnowflakeDate = Date & {
  getEpochSeconds(): number;
  getNanoSeconds(): number;
  getScale(): number;
  /**
   * Timezone of the value:
   * - `string` for TIMESTAMP_NTZ / TIMESTAMP_LTZ (e.g. `'UTC'`, `'America/New_York'`).
   * - `number` for TIMESTAMP_TZ: the offset from UTC in minutes (e.g. `+05:00` -> `300`).
   */
  getTimezone(): string | number;
  getFormat(): string;
};

// TODO: Not yet implemented in the new driver, so falling back to the old one
export type Pool<T> = oldSnowflakeSdk.Pool<T>;
export type FileAndStageBindStatement = oldSnowflakeSdk.FileAndStageBindStatement;
