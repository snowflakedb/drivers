import type { Connection, ConnectionOptions } from 'snowflake-sdk';
import oldSnowflakeSDK from 'snowflake-sdk';
// TODO:
// Ensure tests run against the built package to catch any missing files in the build output
import newSnowflakeSDK from '../../../src/index.js';
import getTestParameter from './getTestParameter';

export function getSnowflakeSDK() {
  if (process.env.SNOWFLAKE_NODEJS_E2E_USE_OLD_DRIVER) {
    return oldSnowflakeSDK;
  } else {
    // TODO:
    // temporary using `as SnowflakeSDK` to satisfy the type checker until
    // new SDK is fully implemented
    return newSnowflakeSDK as typeof oldSnowflakeSDK;
  }
}

export function createConnection(overrides: Partial<ConnectionOptions> = {}): Connection {
  const snowflake = getSnowflakeSDK();
  return snowflake.createConnection({
    account: getTestParameter('SNOWFLAKE_TEST_ACCOUNT'),
    username: getTestParameter('SNOWFLAKE_TEST_USER'),
    password: getTestParameter('SNOWFLAKE_TEST_PASSWORD'),
    warehouse: getTestParameter('SNOWFLAKE_TEST_WAREHOUSE'),
    database: getTestParameter('SNOWFLAKE_TEST_DATABASE'),
    schema: getTestParameter('SNOWFLAKE_TEST_SCHEMA'),
    role: getTestParameter('SNOWFLAKE_TEST_ROLE'),
    ...overrides,
  });
}

export function connectAsync(connection: Connection): Promise<void> {
  return new Promise((resolve, reject) => {
    connection.connect((err) => (err ? reject(err) : resolve()));
  });
}

export function destroyAsync(connection: Connection): Promise<void> {
  return new Promise((resolve, reject) => {
    connection.destroy((err) => (err ? reject(err) : resolve()));
  });
}

export function sleepAsync(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
