import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import type { Connection } from '../../types/sdk-types.js';
import {
  createTestConnection,
  destroyConnectionAsync,
  executeAsync,
  getSnowflakeSDK,
} from '../utils/index.js';

// This file covers only where fetchAsString may be set and which list wins. How a given type
// renders as a string, and what its NULL becomes, lives in tests/e2e/query/data-types/.
describe('fetchAsString', () => {
  const snowflake = getSnowflakeSDK();
  const SQL = 'SELECT 1::NUMBER AS NUM, TRUE::BOOLEAN AS BOOL';

  describe('set per query', () => {
    let connection: Connection;

    beforeAll(async () => {
      connection = createTestConnection(snowflake);
      await connection.connectAsync();
    });

    afterAll(async () => {
      await destroyConnectionAsync(connection);
    });

    it('should return typed values when nothing sets fetchAsString', async () => {
      const { rows } = await executeAsync(connection, SQL);
      expect(rows[0]).toEqual({ NUM: 1, BOOL: true });
    });

    it('should return the listed types as strings and leave the rest typed', async () => {
      const { rows } = await executeAsync(connection, SQL, { fetchAsString: ['Number'] });
      expect(rows[0]).toEqual({ NUM: '1', BOOL: true });
    });
  });

  describe('set on the connection', () => {
    it('should apply to a query that passes no fetchAsString', async () => {
      const connection = createTestConnection(snowflake, { fetchAsString: ['Number'] });
      try {
        await connection.connectAsync();
        const { rows } = await executeAsync(connection, SQL);
        expect(rows[0]).toEqual({ NUM: '1', BOOL: true });
      } finally {
        await destroyConnectionAsync(connection);
      }
    });

    it('should be replaced, not merged, by the fetchAsString a query passes', async () => {
      const connection = createTestConnection(snowflake, { fetchAsString: ['Number'] });
      try {
        await connection.connectAsync();
        const { rows } = await executeAsync(connection, SQL, { fetchAsString: ['Boolean'] });
        expect(rows[0]).toEqual({ NUM: 1, BOOL: 'TRUE' });
      } finally {
        await destroyConnectionAsync(connection);
      }
    });

    it('should be turned off by an empty fetchAsString on the query', async () => {
      const connection = createTestConnection(snowflake, { fetchAsString: ['Number'] });
      try {
        await connection.connectAsync();
        const { rows } = await executeAsync(connection, SQL, { fetchAsString: [] });
        expect(rows[0]).toEqual({ NUM: 1, BOOL: true });
      } finally {
        await destroyConnectionAsync(connection);
      }
    });
  });
});
