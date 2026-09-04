import { describe, it, expect } from 'vitest';
import {
  createTestConnection,
  destroyConnectionAsync,
  executeAsync,
  getSnowflakeSDK,
  NOT_IMPLEMENTED_IN_NEW_DRIVER,
  isRunningNewDriverWithBD,
} from './utils/index.js';

describe('Connection', () => {
  const snowflake = getSnowflakeSDK();

  it('.connect() connects on valid parameters', async () => {
    const connection = createTestConnection(snowflake);
    try {
      await new Promise<void>((resolve, reject) => {
        connection.connect((err) => (err ? reject(err) : resolve()));
      });
      expect(connection.isUp()).toBe(true);
    } finally {
      await destroyConnectionAsync(connection);
    }
  });

  // New driver doesn't have proper error mapping yet
  it.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)(
    '.connect() surfaces an error on invalid parameters',
    async () => {
      const connection = createTestConnection(snowflake, {
        username: 'incorrect-username',
      });
      const error = await new Promise<Error | undefined>((resolve) => {
        connection.connect((err) => resolve(err));
      });
      expect(error).toBeInstanceOf(Error);
      expect(error!.message).toMatch(/JWT token is invalid/);
    },
  );

  it('.connectAsync() connects on valid parameters', async () => {
    const connection = createTestConnection(snowflake);
    try {
      await connection.connectAsync();
      expect(connection.isUp()).toBe(true);
    } finally {
      await destroyConnectionAsync(connection);
    }
  });
  // New driver doesn't have proper error mapping yet
  it.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)(
    '.connectAsync() surfaces an error on invalid parameters',
    async () => {
      const connection = createTestConnection(snowflake, {
        username: 'incorrect-username',
      });
      if (isRunningNewDriverWithBD('BD#11')) {
        await expect(connection.connectAsync()).rejects.toThrow(/JWT token is invalid/);
      } else {
        const error = await new Promise<Error | undefined>((resolve) => {
          void connection.connectAsync((err) => resolve(err));
        });
        expect(error).toBeInstanceOf(Error);
        expect(error!.message).toMatch(/JWT token is invalid/);
      }
    },
  );

  it('destroys the connection and transitions to a disconnected state', async () => {
    const connection = createTestConnection(snowflake);
    await connection.connectAsync();
    await destroyConnectionAsync(connection);
    expect(connection.isUp()).toBe(false);
  });

  it.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)('attaches a query tag from the connection', async () => {
    const expectedQueryTag = 'test_query_tag';
    const connection = createTestConnection(snowflake, { queryTag: expectedQueryTag });
    try {
      await connection.connectAsync();
      const { rows } = await executeAsync(
        connection,
        'SELECT QUERY_TAG FROM table(information_schema.query_history_by_session());',
      );
      expect((rows![0] as Record<string, string>)['QUERY_TAG']).toBe(expectedQueryTag);
    } finally {
      await destroyConnectionAsync(connection);
    }
  });
});
