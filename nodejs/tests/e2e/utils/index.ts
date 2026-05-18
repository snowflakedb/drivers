import type {
  Connection,
  ConnectionOptions,
  FileAndStageBindStatement,
  RowStatement,
  StatementOption,
} from 'snowflake-sdk';
import oldSnowflakeSDK from 'snowflake-sdk';
// TODO:
// Ensure tests run against the built package to catch any missing files in the build output
import newSnowflakeSDK from '../../../src/index.js';
import getTestParameter from './getTestParameter';

export function isRunningForOldDriver() {
  return !!process.env.SNOWFLAKE_NODEJS_E2E_USE_OLD_DRIVER;
}

export function getSnowflakeSDK() {
  if (isRunningForOldDriver()) {
    return oldSnowflakeSDK;
  } else {
    // TODO:
    // temporary using `as SnowflakeSDK` to satisfy the type checker until
    // new SDK is fully implemented
    return newSnowflakeSDK as typeof oldSnowflakeSDK;
  }
}

export const TEST_CONNECTION_OPTIONS: ConnectionOptions = {
  account: getTestParameter('SNOWFLAKE_TEST_ACCOUNT'),
  username: getTestParameter('SNOWFLAKE_TEST_USER'),
  password: getTestParameter('SNOWFLAKE_TEST_PASSWORD'),
  warehouse: getTestParameter('SNOWFLAKE_TEST_WAREHOUSE'),
  database: getTestParameter('SNOWFLAKE_TEST_DATABASE'),
  schema: getTestParameter('SNOWFLAKE_TEST_SCHEMA'),
  role: getTestParameter('SNOWFLAKE_TEST_ROLE'),
};

export function createTestConnection(
  snowflake: typeof oldSnowflakeSDK,
  overrides: Partial<ConnectionOptions> = {},
): Connection {
  return snowflake.createConnection({
    ...TEST_CONNECTION_OPTIONS,
    ...overrides,
  });
}

export function destroyConnectionAsync(connection: Connection): Promise<void> {
  return new Promise((resolve, reject) => {
    connection.destroy((err) => (err ? reject(err) : resolve()));
  });
}

export function executeAsync(
  connection: Connection,
  sqlText: string,
  additionalParameters: Partial<Omit<StatementOption, 'sqlText' | 'complete'>> = {},
): Promise<{
  statement: RowStatement | FileAndStageBindStatement;
  rows: unknown[] | undefined;
}> {
  return new Promise((resolve, reject) => {
    connection.execute({
      sqlText,
      ...additionalParameters,
      complete: (error, statement, rows) => {
        if (error) {
          reject({ error, statement, rows });
        } else {
          resolve({ statement, rows });
        }
      },
    });
  });
}

export function sleepAsync(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
