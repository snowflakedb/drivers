import { afterAll, beforeAll, describe, it, expect, onTestFinished } from 'vitest';
import { Connection } from '../types/sdk-types.js';
import { createConnection, createLiveConnection } from './utils/fixtures.js';
import {
  baseConnectionOptions,
  connectAsyncWithErrorBD,
  destroyConnectionAsync,
  executeAsync,
  snowflake,
  isRunningNewDriverWithBD,
} from './utils/index.js';
import { logoutSuccess, WiremockServer } from './utils/wiremock/index.js';

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
  return snowflake.deserializeConnection(
    { account: baseConnectionOptions.account, host: baseConnectionOptions.host },
    serialized,
  );
}

function serializeConnection(connection: Connection): string {
  return (snowflake.serializeConnection as (connection: Connection) => string)(connection);
}

describe('Connection Serialization & Deserialization', () => {
  describe('serialize()', () => {
    it('should return an empty tokenInfo for a connection that never connected', () => {
      const connection = createConnection();
      expect(connection.serialize()).toBe(PAYLOAD_WITHOUT_TOKENS);
      expect(serializeConnection(connection)).toBe(connection.serialize());
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
      expect(serializeConnection(connection)).toBe(connection.serialize());
    });

    it('should return an empty tokenInfo after the connection is destroyed', async () => {
      const connection = await createLiveConnection({}, false);
      await destroyConnectionAsync(connection);
      expect(connection.serialize()).toBe(PAYLOAD_WITHOUT_TOKENS);
    });
  });

  describe('snowflake.deserializeConnection()', () => {
    it('should return the same tokens before and after the deserialized connection is used', async () => {
      const connection = await createLiveConnection();
      const original = tokenInfoOf(connection.serialize());
      const expected = isRunningNewDriverWithBD('BD#84')
        ? { sessionToken: original.sessionToken, masterToken: original.masterToken }
        : original;
      const deserialized = deserializeConnection(connection.serialize());
      expect(tokenInfoOf(deserialized.serialize())).toEqual(expected);
      await executeAsync(deserialized, 'SELECT 1');
      expect(tokenInfoOf(deserialized.serialize())).toEqual(expected);
    });

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

    it('should not restore a session from a payload without tokens', async () => {
      if (isRunningNewDriverWithBD('BD#71')) {
        expect(() => deserializeConnection(PAYLOAD_WITHOUT_TOKENS)).toThrow(
          expect.objectContaining({
            name: 'InvalidParameterError',
            code: 408003,
            message:
              "Invalid serializedConnection. The value must be a string obtained by calling another connection's serialize() method.",
          }),
        );
      } else {
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
      }
    });

    it('should fail on first use when the session has already been logged out', async () => {
      const connection = await createLiveConnection({}, false);
      const serialized = connection.serialize();
      await destroyConnectionAsync(connection);
      const deserialized = deserializeConnection(serialized);
      await expect(executeAsync(deserialized, SESSION_QUERY)).rejects.toMatchObject(
        isRunningNewDriverWithBD('BD#72')
          ? {
              error: {
                name: 'OperationFailedError',
                code: '390111',
                message: expect.stringContaining('Session no longer exists'),
              },
            }
          : {
              error: {
                name: 'ClientError',
                code: 407002,
                sqlState: '08003',
                message: 'Unable to perform operation using terminated connection.',
                isFatal: true,
              },
            },
      );
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

    it('should throw a missing parameter error when neither account nor host is given', () => {
      const payload = JSON.stringify({
        services: { sf: { tokenInfo: { sessionToken: 'session', masterToken: 'master' } } },
      });
      expect(() => snowflake.deserializeConnection({}, payload)).toThrow(
        expect.objectContaining(
          isRunningNewDriverWithBD('BD#85')
            ? {
                name: 'Error',
                message:
                  "Configuration error: Configuration validation failed (2 issue(s)): account: Missing required parameter 'account'; host: Missing required parameter 'host' (or 'server_url')",
              }
            : {
                name: 'MissingParameterError',
                code: 404007,
                message: 'An account must be specified.',
              },
        ),
      );
    });

    it('should reject an empty JSON object', () => {
      expect(() => deserializeConnection('{}')).toThrow(
        expect.objectContaining(
          isRunningNewDriverWithBD('BD#71')
            ? {
                name: 'InvalidParameterError',
                code: 408003,
                message:
                  "Invalid serializedConnection. The value must be a string obtained by calling another connection's serialize() method.",
              }
            : {
                name: 'InternalAssertError',
                code: 400001,
                message: 'An internal error has occurred. Please contact Snowflake support.',
              },
        ),
      );
    });

    describe('against WireMock', () => {
      let wiremock: WiremockServer;

      beforeAll(async () => {
        wiremock = await WiremockServer.spawn();
      });

      afterAll(async () => {
        await wiremock.destroy();
      });

      it('should restore the session without sending any request', async () => {
        const expirationTime = Date.now() + 60 * 60 * 1000;
        const deserialized = snowflake.deserializeConnection(
          { account: baseConnectionOptions.account, ...wiremock.connectionOptions },
          JSON.stringify({
            services: {
              sf: {
                tokenInfo: {
                  sessionToken: 'session',
                  masterToken: 'master',
                  sessionTokenExpirationTime: expirationTime,
                  masterTokenExpirationTime: expirationTime,
                },
              },
            },
          }),
        );
        onTestFinished(async () => {
          await wiremock.stub(logoutSuccess());
          await destroyConnectionAsync(deserialized);
        });

        expect(deserialized.isUp()).toBe(true);
        expect(await wiremock.findRequests('.*')).toHaveLength(0);
      });
    });
  });
});
