import type { Connection } from 'snowflake-sdk';
import { describe, it, beforeAll, afterAll, expect } from 'vitest';
import { createTestConnection, destroyConnectionAsync, getSnowflakeSDK } from './utils';
import getTestParameter from './utils/getTestParameter';

describe('Connection Serialization', () => {
  const snowflake = getSnowflakeSDK();
  let connection: Connection;

  beforeAll(async () => {
    connection = createTestConnection(snowflake);
    await connection.connectAsync();
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
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
    expect(snowflake.serializeConnection(connection)).toBe(connection.serialize());
  });

  // TODO:
  // Enable after new driver release with a fix:
  // https://github.com/snowflakedb/snowflake-connector-nodejs/pull/1406
  it.skip('snowflake.deserializeConnection() rehydrates into a usable Connection', async () => {
    const snowflake = getSnowflakeSDK();
    const connection2 = snowflake.deserializeConnection(
      {
        account: getTestParameter('SNOWFLAKE_TEST_ACCOUNT'),
        host: getTestParameter('SNOWFLAKE_TEST_HOST'),
      },
      snowflake.serializeConnection(connection),
    );

    try {
      expect(connection2.isUp()).toBe(true);
    } finally {
      await destroyConnectionAsync(connection2);
    }
  });
});
