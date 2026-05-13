import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import oldSnowflakeSDK from 'snowflake-sdk';
import type { Connection, ConnectionOptions, RowStatement } from 'snowflake-sdk';
// TODO:
// Ensure tests run against the built package to catch any missing files in the build output
import newSnowflakeSDK from '../../src/index.js';

export type { Connection, ConnectionOptions, RowStatement };

const PROJECT_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..', '..');

let cachedFileParams: Record<string, string> | null = null;

function loadFileParams(): Record<string, string> | null {
  if (cachedFileParams !== null) return cachedFileParams;

  const parameterPath = process.env.PARAMETER_PATH ?? path.join(PROJECT_ROOT, 'parameters.json');
  if (!fs.existsSync(parameterPath)) return null;

  const raw = JSON.parse(fs.readFileSync(parameterPath, 'utf-8'));
  cachedFileParams = raw.testconnection ?? {};
  return cachedFileParams;
}

export function getTestParameter(key: string): string | undefined {
  return loadFileParams()?.[key] ?? process.env[key];
}

export function getSnowflakeSDK() {
  // SNOWFLAKE_NODEJS_E2E_USE_OLD_DRIVER
  if (process.env.SNOWFLAKE_NODEJS_USE_OLD_DRIVER) {
    return oldSnowflakeSDK;
  } else {
    // TODO:
    // temporary using `as SnowflakeSDK` to satisfy the type checker until
    // new SDK is fully implemented
    return newSnowflakeSDK as typeof oldSnowflakeSDK;
  }
}

export function sleepAsync(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
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
