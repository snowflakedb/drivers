import type { Connection } from 'snowflake-sdk';
import { describe, it, beforeEach, afterEach, expect } from 'vitest';
import { createTestConnection, destroyConnectionAsync, getSnowflakeSDK } from './utils';

describe('Connection', () => {
  const snowflake = getSnowflakeSDK();
  let connection: Connection;

  beforeEach(async () => {
    connection = createTestConnection(snowflake);
  });

  afterEach(async () => {
    await destroyConnectionAsync(connection);
  });

  it('connects using .connect()', async () => {
    await new Promise((resolve, reject) => {
      connection.connect((err) => (err ? reject(err) : resolve(null)));
    });
    expect(connection.isUp()).toBe(true);
  });

  it('connects using .connectAsync()', async () => {
    await connection.connectAsync();
    expect(connection.isUp()).toBe(true);
  });
});
