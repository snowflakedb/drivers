import type { Connection } from 'snowflake-sdk';
import { describe, it, beforeAll, afterAll } from 'vitest';
import { createTestConnection, destroyConnectionAsync, getSnowflakeSDK, sleepAsync } from './utils';

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
    await sleepAsync(2000);
    await new Promise<void>((resolve, reject) => {
      statement.cancel((err) => (err ? reject(err) : resolve()));
    });
  });
});
