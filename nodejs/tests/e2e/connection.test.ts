import { describe, it, expect } from 'vitest';
import {
  createTestConnection,
  destroyConnectionAsync,
  NOT_IMPLEMENTED_IN_NEW_DRIVER,
  isRunningNewDriverWithBD,
} from './utils/index.js';

describe('Connection', () => {
  it('.connect() connects on valid parameters', async () => {
    const connection = createTestConnection();
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
      const connection = createTestConnection({
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
    const connection = createTestConnection();
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
      const connection = createTestConnection({
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
    const connection = createTestConnection();
    await connection.connectAsync();
    await destroyConnectionAsync(connection);
    expect(connection.isUp()).toBe(false);
  });

  describe('isValidAsync', () => {
    it('should report a connection as valid after connect', async () => {
      const connection = createTestConnection();
      try {
        await connection.connectAsync();
        await expect(connection.isValidAsync()).resolves.toBe(true);
      } finally {
        await destroyConnectionAsync(connection);
      }
    });

    it('should report a connection as invalid after destroy', async () => {
      const connection = createTestConnection();
      await connection.connectAsync();
      await destroyConnectionAsync(connection);
      await expect(connection.isValidAsync()).resolves.toBe(false);
    });
  });
});
