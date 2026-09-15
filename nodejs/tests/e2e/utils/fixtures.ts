import { onTestFinished } from 'vitest';
import { Connection, ConnectionOptions } from '../../types/sdk-types.js';
import { TempDir, type TempDirOptions } from './files.js';
import {
  destroyConnectionAsync,
  executeAsync,
  randomizeName,
  snowflake,
  TEST_CONNECTION_OPTIONS,
} from './index.js';

export function createConnection(overrides: Partial<ConnectionOptions> = {}): Connection {
  return snowflake.createConnection({
    ...TEST_CONNECTION_OPTIONS,
    ...overrides,
  });
}

export async function createLiveConnection(
  overrides: Partial<ConnectionOptions> = {},
  shouldCleanupAfterTest = true,
): Promise<Connection> {
  const connection = createConnection(overrides);
  await connection.connectAsync();
  if (shouldCleanupAfterTest) {
    onTestFinished(async () => {
      await destroyConnectionAsync(connection);
    });
  }
  return connection;
}

export async function createTemporaryTable(
  connection: Connection,
  columns: string,
  shouldCleanupAfterTest = true,
): Promise<string> {
  const tableName = randomizeName('nodejs_');
  await executeAsync(connection, `CREATE OR REPLACE TEMPORARY TABLE ${tableName} (${columns})`);
  if (shouldCleanupAfterTest) {
    onTestFinished(async () => {
      await executeAsync(connection, `DROP TABLE IF EXISTS ${tableName}`);
    });
  }
  return tableName;
}

export function createTempDir(
  options: TempDirOptions = {},
  shouldCleanupAfterTest = true,
): TempDir {
  const tempDir = new TempDir(options);
  if (shouldCleanupAfterTest) {
    onTestFinished(() => {
      tempDir.cleanup();
    });
  }
  return tempDir;
}

export interface TemporaryStageOptions {
  encryption?: string;
  directory?: boolean;
}

export async function createTemporaryStage(
  connection: Connection,
  options: TemporaryStageOptions = {},
  shouldCleanupAfterTest = true,
): Promise<string> {
  const stageName = randomizeName('nodejs_stage_');
  const encryptionClause = options.encryption
    ? ` ENCRYPTION = (TYPE = '${options.encryption}')`
    : '';
  const directoryClause =
    options.directory === undefined
      ? ''
      : ` DIRECTORY = (ENABLE = ${options.directory ? 'TRUE' : 'FALSE'})`;
  await executeAsync(
    connection,
    `CREATE TEMPORARY STAGE IF NOT EXISTS ${stageName}${encryptionClause}${directoryClause}`,
  );
  if (shouldCleanupAfterTest) {
    onTestFinished(async () => {
      await executeAsync(connection, `DROP STAGE IF EXISTS ${stageName}`);
    });
  }
  return stageName;
}
