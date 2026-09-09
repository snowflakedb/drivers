import { describe, it, expect } from 'vitest';
import { createConnection, createLiveConnection } from './utils/fixtures.js';
import getTestParameter from './utils/getTestParameter.js';
import { destroyConnectionAsync, snowflake, NOT_IMPLEMENTED_IN_NEW_DRIVER } from './utils/index.js';

describe.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)('Connection Serialization & Deserialization', () => {
  it('serialization of a disconnected connection returns empty tokenInfo', () => {
    const disconnectedConnection = createConnection();
    const serialized = disconnectedConnection.serialize();
    // @ts-ignore NOT_IMPLEMENTED_IN_NEW_DRIVER
    expect(snowflake.serializeConnection(disconnectedConnection)).toEqual(serialized);
    expect(JSON.parse(serialized)).toEqual({
      services: { sf: { tokenInfo: {} } },
    });
  });

  it('connection.serialize() returns a JSON string with services.sf.tokenInfo', async () => {
    const connection = await createLiveConnection();
    const serialized = connection.serialize();
    expect(typeof serialized).toBe('string');
    expect(serialized.length).toBeGreaterThan(0);

    const tokenInfo = JSON.parse(serialized)?.services?.sf?.tokenInfo;
    expect(tokenInfo).toBeTruthy();
    expect(typeof tokenInfo.masterToken).toBe('string');
    expect(typeof tokenInfo.sessionToken).toBe('string');
    expect(typeof tokenInfo.masterTokenExpirationTime).toBe('number');
    expect(typeof tokenInfo.sessionTokenExpirationTime).toBe('number');
  });

  it('snowflake.serializeConnection() returns the same string as connection.serialize()', async () => {
    const connection = await createLiveConnection();
    // @ts-ignore NOT_IMPLEMENTED_IN_NEW_DRIVER
    expect(snowflake.serializeConnection(connection)).toBe(connection.serialize());
  });

  describe('snowflake.deserializeConnection()', () => {
    it('rehydrates into a usable connection', async () => {
      const connection = await createLiveConnection();
      // @ts-ignore NOT_IMPLEMENTED_IN_NEW_DRIVER
      const connectionFromDeserialization = snowflake.deserializeConnection(
        {
          account: getTestParameter('SNOWFLAKE_TEST_ACCOUNT'),
          host: getTestParameter('SNOWFLAKE_TEST_HOST'),
        },
        // @ts-ignore NOT_IMPLEMENTED_IN_NEW_DRIVER
        snowflake.serializeConnection(connection),
      );

      try {
        expect(connectionFromDeserialization.isUp()).toBe(true);
      } finally {
        await destroyConnectionAsync(connectionFromDeserialization);
      }
    });

    it('rehydrates into a disconnected connection when tokens are missing', () => {
      // @ts-ignore NOT_IMPLEMENTED_IN_NEW_DRIVER
      const connectionFromDeserialization = snowflake.deserializeConnection(
        {
          account: getTestParameter('SNOWFLAKE_TEST_ACCOUNT'),
          host: getTestParameter('SNOWFLAKE_TEST_HOST'),
        },
        JSON.stringify({ services: { sf: { tokenInfo: {} } } }),
      );
      expect(connectionFromDeserialization.isUp()).toBe(false);
    });
  });
});
