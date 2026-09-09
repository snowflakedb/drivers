import { describe, it, expect } from 'vitest';
import { Connection } from '../types/sdk-types.js';
import { createConnection, createLiveConnection } from './utils/fixtures.js';
import { destroyConnectionAsync, executeAsync, isRunningNewDriverWithBD } from './utils/index.js';

function connectAsyncWithErrorBD(connection: Connection) {
  if (isRunningNewDriverWithBD('BD#11')) {
    return connection.connectAsync();
  } else {
    return new Promise((resolve, reject) => {
      connection.connect((error: unknown) => {
        if (error) {
          reject(error);
        } else {
          resolve(void 0);
        }
      });
    });
  }
}

describe('Connection State Errors', () => {
  it('rejects a statement issued before the connection is established', async () => {
    const connection = createConnection();
    await expect(executeAsync(connection, 'select 1')).rejects.toMatchObject({
      error: {
        name: 'ClientError',
        code: 407001,
        sqlState: '08003',
        message: 'Unable to perform operation because a connection was never established.',
      },
    });
  });

  it('rejects a statement issued after the connection is destroyed', async () => {
    const connection = await createLiveConnection({}, false);
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
    const connection = createConnection({
      username: 'no_such_user_for_e2e',
    });
    await expect(connectAsyncWithErrorBD(connection)).rejects.toThrow();
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

  it('should refuse to destroy a connection that was never established', async () => {
    const connection = createConnection();
    await expect(destroyConnectionAsync(connection)).rejects.toMatchObject({
      name: 'ClientError',
      code: 406501,
      message: 'Not connected, so nothing to destroy.',
    });
  });

  it('should refuse to destroy a connection that is already destroyed', async () => {
    const connection = await createLiveConnection({}, false);
    await destroyConnectionAsync(connection);
    await expect(destroyConnectionAsync(connection)).rejects.toMatchObject({
      name: 'ClientError',
      code: 406502,
      message: 'Already disconnected.',
    });
  });

  it('should refuse to destroy a connection whose login failed', async () => {
    const connection = createConnection({
      username: 'no_such_user_for_e2e',
    });
    await expect(connectAsyncWithErrorBD(connection)).rejects.toThrow();
    await expect(destroyConnectionAsync(connection)).rejects.toMatchObject({
      name: 'ClientError',
      code: 406502,
      message: 'Already disconnected.',
    });
  });
});
