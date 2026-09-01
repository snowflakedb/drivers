import { describe, it, beforeAll, afterAll, expect } from 'vitest';
import type { Connection, SnowflakeError } from '../types/sdk-types.js';
import {
  createTestConnection,
  destroyConnectionAsync,
  executeAsync,
  getSnowflakeSDK,
} from './utils/index.js';

describe('Query Errors', () => {
  const snowflake = getSnowflakeSDK();
  let connection: Connection;

  beforeAll(async () => {
    connection = createTestConnection(snowflake);
    await connection.connectAsync();
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  it('reports a statement the server rejected as an operation failure', async () => {
    const execution = executeAsync(connection, 'select * from a_table_that_does_not_exist');

    await expect(execution).rejects.toMatchObject({
      error: {
        name: 'OperationFailedError',
        code: '002003',
        sqlState: '42S02',
        message: expect.stringContaining('does not exist or not authorized'),
      } satisfies Partial<SnowflakeError>,
    });
  });
});
