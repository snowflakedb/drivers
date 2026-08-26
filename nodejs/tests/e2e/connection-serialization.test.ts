import { describe, it, beforeAll, afterAll, expect } from 'vitest';
import type { Connection } from '../types/sdk-types.js';
import getTestParameter from './utils/getTestParameter.js';
import {
  createTestConnection,
  destroyConnectionAsync,
  getSnowflakeSDK,
  NOT_IMPLEMENTED_IN_NEW_DRIVER,
} from './utils/index.js';

describe.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)('Connection Serialization & Deserialization', () => {
  const snowflake = getSnowflakeSDK();
  let connection: Connection;

  beforeAll(async () => {
    connection = createTestConnection(snowflake);
    await connection.connectAsync();
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  it('serialization of a disconnected connection returns empty tokenInfo', () => {
    const disconnectedConnection = createTestConnection(snowflake);
    const serialized = disconnectedConnection.serialize();
    // @ts-ignore NOT_IMPLEMENTED_IN_NEW_DRIVER
    expect(snowflake.serializeConnection(disconnectedConnection)).toEqual(serialized);
    expect(JSON.parse(serialized)).toEqual({
      services: { sf: { tokenInfo: {} } },
    });
  });

  it('connection.serialize() returns a JSON string with services.sf.tokenInfo', () => {
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

  it('snowflake.serializeConnection() returns the same string as connection.serialize()', () => {
    const snowflake = getSnowflakeSDK();
    // @ts-ignore NOT_IMPLEMENTED_IN_NEW_DRIVER
    expect(snowflake.serializeConnection(connection)).toBe(connection.serialize());
  });

  describe('snowflake.deserializeConnection()', () => {
    it('rehydrates into a usable connection', async () => {
      const snowflake = getSnowflakeSDK();
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
      const snowflake = getSnowflakeSDK();
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
