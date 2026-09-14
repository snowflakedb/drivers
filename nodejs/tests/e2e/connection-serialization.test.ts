import { describe, it, expect } from 'vitest';
import { Connection } from '../types/sdk-types.js';
import { createConnection, createLiveConnection } from './utils/fixtures.js';
import {
  baseConnectionOptions,
  connectAsyncWithErrorBD,
  destroyConnectionAsync,
  executeAsync,
  snowflake,
  NOT_IMPLEMENTED_IN_NEW_DRIVER,
} from './utils/index.js';

const PAYLOAD_WITHOUT_TOKENS = JSON.stringify({ services: { sf: { tokenInfo: {} } } });
const SESSION_QUERY = 'select current_session() as session';

type TokenInfo = {
  sessionToken?: string;
  masterToken?: string;
  sessionTokenExpirationTime?: number;
  masterTokenExpirationTime?: number;
};

function tokenInfoOf(serializedConnection: string): TokenInfo {
  return JSON.parse(serializedConnection)?.services?.sf?.tokenInfo;
}

async function sessionIdOf(connection: Connection): Promise<unknown> {
  const { rows } = await executeAsync(connection, SESSION_QUERY);
  return rows[0].SESSION;
}

function deserializeConnection(serialized: string): Connection {
  // @ts-ignore NOT_IMPLEMENTED_IN_NEW_DRIVER
  return snowflake.deserializeConnection(
    { account: baseConnectionOptions.account, host: baseConnectionOptions.host },
    serialized,
  );
}

describe.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)('Connection Serialization & Deserialization', () => {
  describe('serialize()', () => {
    it('should return an empty tokenInfo for a connection that never connected', () => {
      const connection = createConnection();
      expect(connection.serialize()).toBe(PAYLOAD_WITHOUT_TOKENS);
      // @ts-ignore NOT_IMPLEMENTED_IN_NEW_DRIVER
      expect(snowflake.serializeConnection(connection)).toBe(connection.serialize());
    });

    it('should return both tokens with an expiration time each', async () => {
      const connection = await createLiveConnection();
      const tokenInfo = tokenInfoOf(connection.serialize());
      expect(tokenInfo.sessionToken).toBeTypeOf('string');
      expect(tokenInfo.masterToken).toBeTypeOf('string');
      expect(tokenInfo.sessionTokenExpirationTime).toBeTypeOf('number');
      expect(tokenInfo.masterTokenExpirationTime).toBeTypeOf('number');
      // The master token outlives the session token it exists to renew.
      expect(tokenInfo.masterTokenExpirationTime!).toBeGreaterThan(
        tokenInfo.sessionTokenExpirationTime!,
      );
    });

    it('should return the same string as snowflake.serializeConnection()', async () => {
      const connection = await createLiveConnection();
      // @ts-ignore NOT_IMPLEMENTED_IN_NEW_DRIVER
      expect(snowflake.serializeConnection(connection)).toBe(connection.serialize());
    });

    it('should return an empty tokenInfo after the connection is destroyed', async () => {
      const connection = await createLiveConnection({}, false);
      await destroyConnectionAsync(connection);
      expect(connection.serialize()).toBe(PAYLOAD_WITHOUT_TOKENS);
    });
  });

  describe('snowflake.deserializeConnection()', () => {
    it('should deserialize into the originating session and leave it usable', async () => {
      const connection = await createLiveConnection();
      const originalSessionId = await sessionIdOf(connection);
      const deserialized = deserializeConnection(connection.serialize());
      expect(deserialized.isUp()).toBe(true);
      expect(await sessionIdOf(deserialized)).toBe(originalSessionId);
      expect(await sessionIdOf(connection)).toBe(originalSessionId);
    });

    it('should refuse a connect() on a deserialized connection', async () => {
      const connection = await createLiveConnection();
      const deserialized = deserializeConnection(connection.serialize());
      await expect(connectAsyncWithErrorBD(deserialized)).rejects.toMatchObject({
        name: 'ClientError',
        code: 405502,
        sqlState: '08002',
        message: 'Already connected.',
      });
    });

    it('should deserialize a disconnected connection when the tokens are missing', async () => {
      const deserialized = deserializeConnection(PAYLOAD_WITHOUT_TOKENS);
      expect(deserialized.isUp()).toBe(false);
      await expect(executeAsync(deserialized, SESSION_QUERY)).rejects.toMatchObject({
        error: {
          name: 'ClientError',
          code: 407001,
          sqlState: '08003',
          message: 'Unable to perform operation because a connection was never established.',
        },
      });
    });

    it('should fail on first use when the session has already been logged out', async () => {
      const connection = await createLiveConnection({}, false);
      const serialized = connection.serialize();
      await destroyConnectionAsync(connection);
      const deserialized = deserializeConnection(serialized);
      await expect(executeAsync(deserialized, SESSION_QUERY)).rejects.toMatchObject({
        error: {
          name: 'ClientError',
          code: 407002,
          sqlState: '08003',
          message: 'Unable to perform operation using terminated connection.',
          isFatal: true,
        },
      });
    });

    it.each([
      ['a string that is not JSON', 'not json at all'],
      ['a JSON number', '123'],
      ['a JSON array', '[]'],
    ])('should reject %s', (_description, payload) => {
      expect(() => deserializeConnection(payload)).toThrow(
        expect.objectContaining({
          name: 'InvalidParameterError',
          code: 408003,
          message:
            "Invalid serializedConnection. The value must be a string obtained by calling another connection's serialize() method.",
        }),
      );
    });
  });
});
