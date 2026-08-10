import type {
  Connection,
  ConnectionOptions,
  FileAndStageBindStatement,
  RowStatement,
  StatementOption,
} from 'snowflake-sdk';
import oldSnowflakeSDK from 'snowflake-sdk';
// TODO:
// Ensure tests run against the built package to catch any missing files in the build output.
// Namespace import (not default): src/index.js only has named exports. A default import
// works under Vitest interop but fails under plain Node/tsx with
// "does not provide an export named 'default'".
import * as newSnowflakeSDK from '../../../src/index.js';
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

const _baseConnectionOptions = {
  account: getTestParameter('SNOWFLAKE_TEST_ACCOUNT'),
  host: getTestParameter('SNOWFLAKE_TEST_HOST'),
  username: getTestParameter('SNOWFLAKE_TEST_USER'),
  warehouse: getTestParameter('SNOWFLAKE_TEST_WAREHOUSE'),
  database: getTestParameter('SNOWFLAKE_TEST_DATABASE'),
  schema: getTestParameter('SNOWFLAKE_TEST_SCHEMA'),
  role: getTestParameter('SNOWFLAKE_TEST_ROLE'),
};

export const TEST_CONNECTION_OPTIONS: ConnectionOptions = {
  ..._baseConnectionOptions,
  authenticator: 'SNOWFLAKE_JWT',
  privateKey: getTestParameter('SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS'),
  privateKeyPass: getTestParameter('SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD'),
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
  rows: Record<string, unknown>[];
}> {
  return new Promise((resolve, reject) => {
    connection.execute({
      sqlText,
      ...additionalParameters,
      complete: (error, statement, rows) => {
        if (error) {
          reject({ error, statement });
        } else {
          resolve({ statement, rows: rows as Record<string, unknown>[] });
        }
      },
    });
  });
}

export function sleepAsync(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
