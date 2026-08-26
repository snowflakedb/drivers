import type {
  Connection,
  ConnectionOptions,
  FileAndStageBindStatement,
  RowStatement,
  StatementOption,
} from 'snowflake-sdk-old';
import { randomUUID } from 'node:crypto';
import oldSnowflakeSDK from 'snowflake-sdk-old';
import getTestParameter from './getTestParameter.js';

const IS_RUNNING_FOR_OLD_DRIVER = !!process.env.SNOWFLAKE_NODEJS_E2E_USE_OLD_DRIVER;

// Dynamic `import()` so old-driver e2e never loads the new SDK or its native core.
// TODO: drop the cast once the new driver's public types match the old SDK.
const newSnowflakeSDK = IS_RUNNING_FOR_OLD_DRIVER
  ? undefined
  : ((await import('snowflake-sdk')) as unknown as typeof oldSnowflakeSDK);

/**
 * @deprecated Do not use this function in tests. Use isRunningNewDriverWithBD instead.
 */
export function isRunningForOldDriver() {
  return IS_RUNNING_FOR_OLD_DRIVER;
}

/**
 * Marks a documented behavior difference between the old and new Node.js
 * driver, letting a test branch its assertions on which driver is under test.
 *
 * `bdRef` is a `BD#<n>` reference whose number must match an entry key in
 * `nodejs/BehaviorDifferences.yaml`, so the divergence a test relies on is always traceable to a
 * documented, reviewed behavior difference.
 */
export function isRunningNewDriverWithBD(bdRef: `BD#${number}`): boolean {
  void bdRef;
  return !IS_RUNNING_FOR_OLD_DRIVER;
}

export function getSnowflakeSDK() {
  if (IS_RUNNING_FOR_OLD_DRIVER) {
    return oldSnowflakeSDK;
  } else {
    return newSnowflakeSDK!;
  }
}

const baseConnectionOptions = {
  account: getTestParameter('SNOWFLAKE_TEST_ACCOUNT'),
  host: getTestParameter('SNOWFLAKE_TEST_HOST'),
  username: getTestParameter('SNOWFLAKE_TEST_USER'),
  warehouse: getTestParameter('SNOWFLAKE_TEST_WAREHOUSE'),
  database: getTestParameter('SNOWFLAKE_TEST_DATABASE'),
  schema: getTestParameter('SNOWFLAKE_TEST_SCHEMA'),
  role: getTestParameter('SNOWFLAKE_TEST_ROLE'),
};

export const TEST_CONNECTION_OPTIONS: ConnectionOptions = getTestParameter('SNOWFLAKE_TEST_IS_USUT')
  ? {
      ...baseConnectionOptions,
      password: getTestParameter('SNOWFLAKE_TEST_PASSWORD'),
    }
  : {
      ...baseConnectionOptions,
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

export function randomizeName(prefix: string): string {
  return `${prefix}${randomUUID().replaceAll('-', '')}`;
}
