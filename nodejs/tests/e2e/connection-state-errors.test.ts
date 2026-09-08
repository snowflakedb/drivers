import { describe, it, expect } from 'vitest';
import type { Connection } from '../types/sdk-types.js';
import { createTestConnection, destroyConnectionAsync, executeAsync } from './utils/index.js';

describe('Connection State Errors', () => {
  const failLogin = async (connection: Connection): Promise<void> => {
    // connectAsync reports login failures differently on the two drivers (BD#11).
    const loginError = await new Promise<unknown>((resolve) => {
      connection.connect((error) => resolve(error));
    });
    expect(loginError).toBeInstanceOf(Error);
  };

  const destroyError = (connection: Connection): Promise<unknown> =>
    new Promise((resolve) => {
      connection.destroy((error) => resolve(error));
    });

  it('rejects a statement issued before the connection is established', async () => {
    const connection = createTestConnection();

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
      await destroyConnectionAsync(connection);
    }
  });

  it('rejects a statement issued after the connection is destroyed', async () => {
    const connection = createTestConnection();
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
    const connection = createTestConnection({
      username: 'no_such_user_for_e2e',
    });
    await failLogin(connection);

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
      await destroyConnectionAsync(connection);
    }
  });

  it('should refuse to destroy a connection that was never established', async () => {
    const connection = createTestConnection();

    await expect(destroyError(connection)).resolves.toMatchObject({
      name: 'ClientError',
      code: 406501,
      message: 'Not connected, so nothing to destroy.',
    });
  });

  it('should refuse to destroy a connection that is already destroyed', async () => {
    const connection = createTestConnection();
    await connection.connectAsync();
    await destroyConnectionAsync(connection);

    await expect(destroyError(connection)).resolves.toMatchObject({
      name: 'ClientError',
      code: 406502,
      message: 'Already disconnected.',
    });
  });

  it('should refuse to destroy a connection whose login failed', async () => {
    const connection = createTestConnection({
      username: 'no_such_user_for_e2e',
    });
    await failLogin(connection);

    await expect(destroyError(connection)).resolves.toMatchObject({
      name: 'ClientError',
      code: 406502,
      message: 'Already disconnected.',
    });
  });
});
