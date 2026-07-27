import type { Connection } from 'snowflake-sdk';
import { describe, it, expect } from 'vitest';
import {
  createTestConnection,
  destroyConnectionAsync,
  executeAsync,
  getSnowflakeSDK,
  isRunningForOldDriver,
} from './utils';

describe('Connection', () => {
  const snowflake = getSnowflakeSDK();

  describe.each([
    {
      name: '.connect()',
      connectFn: (connection: Connection) => {
        return new Promise<void>((resolve, reject) => {
          connection.connect((err) => (err ? reject(err) : resolve()));
        });
      },
    },
    {
      name: '.connectAsync()',
      connectFn: (connection: Connection) => connection.connectAsync(),
    },
  ])('$name', ({ name, connectFn }) => {
    it('connects on valid parameters', async () => {
      const connection = createTestConnection(snowflake);
      try {
        await connectFn(connection);
        expect(connection.isUp()).toBe(true);
      } finally {
        await destroyConnectionAsync(connection);
      }
    });

    (name === '.connectAsync()' && isRunningForOldDriver() ? it.skip : it)(
      'surfaces an error on invalid parameters',
      async () => {
        const connection = createTestConnection(snowflake, {
          username: 'incorrect-username',
        });
        const error = await connectFn(connection).catch((err) => err);
        expect(error.message).toMatch(/JWT token is invalid/);
      },
    );
  });

  it('destroys the connection and transitions to a disconnected state', async () => {
    const connection = createTestConnection(snowflake);
    await connection.connectAsync();
    await destroyConnectionAsync(connection);
    expect(connection.isUp()).toBe(false);
  });

  it('attaches a query tag from the connection', async () => {
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
