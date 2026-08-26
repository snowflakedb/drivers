import type { Connection, RowStatement } from 'snowflake-sdk-old';
import { describe, it, beforeAll, afterAll, expect } from 'vitest';
import {
  createTestConnection,
  destroyConnectionAsync,
  executeAsync,
  getSnowflakeSDK,
  sleepAsync,
} from './utils/index.js';

function cancelStatement(statement: RowStatement) {
  return new Promise<void>((resolve, reject) => {
    statement.cancel((err) => (err ? reject(err) : resolve()));
  });
}

describe('Query Cancellation', () => {
  const snowflake = getSnowflakeSDK();
  let connection: Connection;

  beforeAll(async () => {
    connection = createTestConnection(snowflake);
    await connection.connectAsync();
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  it('cancels a running query', async () => {
    const statement = connection.execute({
      sqlText: 'select count(*) from table(generator(timeLimit => 3600))',
    });
    await sleepAsync(2000); // wait for query to start running
    await cancelStatement(statement);
  });

  it('throws when failing to cancel a query', async () => {
    const { statement } = await executeAsync(connection, 'select 1');
    // Query is completed = nothing to cancel
    await expect(cancelStatement(statement)).rejects.toMatchObject({
      code: '000605',
      sqlState: '01000',
    });
  });
});
