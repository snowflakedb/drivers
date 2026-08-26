import * as newSnowflakeSdk from 'snowflake-sdk';
import oldSnowflakeSdk from 'snowflake-sdk-old';

export type Connection = newSnowflakeSdk.Connection | oldSnowflakeSdk.Connection;
export type Pool<T> = newSnowflakeSdk.Pool<T> | oldSnowflakeSdk.Pool<T>;
export type ConnectionOptions =
  | newSnowflakeSdk.ConnectionOptions
  | oldSnowflakeSdk.ConnectionOptions;
export type FileAndStageBindStatement =
  | newSnowflakeSdk.FileAndStageBindStatement
  | oldSnowflakeSdk.FileAndStageBindStatement;
export type RowStatement = newSnowflakeSdk.RowStatement | oldSnowflakeSdk.RowStatement;
export type StatementOption = newSnowflakeSdk.StatementOption | oldSnowflakeSdk.StatementOption;
export type QueryStatus = newSnowflakeSdk.QueryStatus | oldSnowflakeSdk.QueryStatus;
export type SnowflakeError = newSnowflakeSdk.SnowflakeError | oldSnowflakeSdk.SnowflakeError;
export type RowMode = newSnowflakeSdk.RowMode | oldSnowflakeSdk.RowMode;
