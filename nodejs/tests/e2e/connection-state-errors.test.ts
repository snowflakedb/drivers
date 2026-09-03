import { describe, it, expect } from 'vitest';
import type { Connection } from '../types/sdk-types.js';
import {
  createTestConnection,
  destroyConnectionAsync,
  executeAsync,
  getSnowflakeSDK,
} from './utils/index.js';

describe('Connection State Errors', () => {
  const snowflake = getSnowflakeSDK();

  // The old driver refuses to destroy a connection in these states (406501,
  // 406502); ours accepts it, so releasing the handle is best effort here.
  const releaseConnection = (connection: Connection) =>
    destroyConnectionAsync(connection).catch(() => {});

  it('rejects a statement issued before the connection is established', async () => {
    const connection = createTestConnection(snowflake);

    try {
      await expect(executeAsync(connection, 'select 1')).rejects.toMatchObject({
        error: {
          name: 'ClientError',
          code: 407001,
          sqlState: '08003',
          message: 'Unable to perform operation because a connection was never established.',
        },
      });
    } finally {
      await releaseConnection(connection);
    }
  });

  it('rejects a statement issued after the connection is destroyed', async () => {
    const connection = createTestConnection(snowflake);
    await connection.connectAsync();
    await destroyConnectionAsync(connection);

    await expect(executeAsync(connection, 'select 1')).rejects.toMatchObject({
      error: {
        name: 'ClientError',
        code: 407002,
        sqlState: '08003',
        message: 'Unable to perform operation using terminated connection.',
        isFatal: true,
      },
    });
  });

  it('rejects a statement issued after the login failed', async () => {
    const connection = createTestConnection(snowflake, {
      username: 'no_such_user_for_e2e',
    });

    // connectAsync reports login failures differently on the two drivers (BD#11).
    const loginError = await new Promise<unknown>((resolve) => {
      connection.connect((error) => resolve(error));
    });
    expect(loginError).toBeInstanceOf(Error);

    try {
      await expect(executeAsync(connection, 'select 1')).rejects.toMatchObject({
        error: {
          name: 'ClientError',
          code: 407002,
          sqlState: '08003',
          message: 'Unable to perform operation using terminated connection.',
          isFatal: true,
        },
      });
    } finally {
      await releaseConnection(connection);
    }
  });
});
