import * as newSnowflakeSdk from 'snowflake-sdk';
import oldSnowflakeSdk from 'snowflake-sdk-old';

export type Connection = newSnowflakeSdk.Connection | oldSnowflakeSdk.Connection;
export type ConnectionOptions =
  | newSnowflakeSdk.ConnectionOptions
  | oldSnowflakeSdk.ConnectionOptions;
export type RowStatement = newSnowflakeSdk.RowStatement | oldSnowflakeSdk.RowStatement;
export type StatementOption = newSnowflakeSdk.StatementOption | oldSnowflakeSdk.StatementOption;
export type Binds = newSnowflakeSdk.Binds | oldSnowflakeSdk.Binds;
export type SnowflakeError = newSnowflakeSdk.SnowflakeError | oldSnowflakeSdk.SnowflakeError;
export type RowMode = newSnowflakeSdk.RowMode | oldSnowflakeSdk.RowMode;

// TODO: Not yet implemented in the new driver, so falling back to the old one
export type Pool<T> = oldSnowflakeSdk.Pool<T>;
export type FileAndStageBindStatement = oldSnowflakeSdk.FileAndStageBindStatement;
export type QueryStatus = oldSnowflakeSdk.QueryStatus;
