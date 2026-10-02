import * as newSnowflakeSdk from 'snowflake-sdk';
import oldSnowflakeSdk from 'snowflake-sdk-old';

// Available only in new driver (BD#48)
export type { SnowflakeDate } from 'snowflake-sdk';

export type NewSnowflakeSdkColumn = newSnowflakeSdk.Column;

export type SnowflakeError = newSnowflakeSdk.SnowflakeError | oldSnowflakeSdk.SnowflakeError;

export type Connection = newSnowflakeSdk.Connection | oldSnowflakeSdk.Connection;
export type ConnectionOptions =
  | newSnowflakeSdk.ConnectionOptions
  | oldSnowflakeSdk.ConnectionOptions;

export type RowStatement = newSnowflakeSdk.RowStatement | oldSnowflakeSdk.RowStatement;
export type StatementOption = newSnowflakeSdk.StatementOption | oldSnowflakeSdk.StatementOption;
export type FetchResultOptions = newSnowflakeSdk.FetchResultOptions;
export type Binds = newSnowflakeSdk.Binds | oldSnowflakeSdk.Binds;
export type RowMode = newSnowflakeSdk.RowMode | oldSnowflakeSdk.RowMode;
export type QueryStatus = newSnowflakeSdk.QueryStatus | oldSnowflakeSdk.QueryStatus;

export type SessionState = newSnowflakeSdk.SessionState;
export type Pool<T> = newSnowflakeSdk.Pool<T> | oldSnowflakeSdk.Pool<T>;
export type FileAndStageBindStatement = oldSnowflakeSdk.FileAndStageBindStatement;
