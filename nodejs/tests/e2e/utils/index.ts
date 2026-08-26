import type { RowStatement as OldRowStatement } from 'snowflake-sdk-old';
import { randomUUID } from 'node:crypto';
import newSnowflakeSDK from 'snowflake-sdk';
import oldSnowflakeSDK from 'snowflake-sdk-old';
import type {
  Connection,
  ConnectionOptions,
  FileAndStageBindStatement,
  RowStatement,
  StatementOption,
} from '../../types/sdk-types.js';
import getTestParameter from './getTestParameter.js';

const IS_RUNNING_FOR_OLD_DRIVER = !!process.env.SNOWFLAKE_NODEJS_E2E_USE_OLD_DRIVER;

// A var to use in .skipIf() vitest conditionals while some features are not implemented in the new driver
export const NOT_IMPLEMENTED_IN_NEW_DRIVER = !IS_RUNNING_FOR_OLD_DRIVER;

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
    return newSnowflakeSDK;
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
  snowflake: ReturnType<typeof getSnowflakeSDK>,
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

// TODO: remove once we drop old-driver e2e tests. Every test should then call
// statement.getColumn(...) directly instead of this helper.
export function getStatementColumn(statement: RowStatement, id: string | number) {
  if (isRunningNewDriverWithBD('BD#13')) {
    return statement.getColumn(id)!;
  }
  return (statement as OldRowStatement).getColumn(id);
}

export function sleepAsync(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export function randomizeName(prefix: string): string {
  return `${prefix}${randomUUID().replaceAll('-', '')}`;
}
