import { describe, expect, it } from 'vitest';
import { createLiveConnection } from '../utils/fixtures.js';
import { executeAsync } from '../utils/index.js';

// This file covers only where fetchAsString may be set and which list wins. How a given type
// renders as a string, and what its NULL becomes, lives in tests/e2e/query/data-types/.
describe('fetchAsString', () => {
  const SQL = 'SELECT 1::NUMBER AS NUM, TRUE::BOOLEAN AS BOOL';

  describe('set per query', () => {
    it('should return typed values when nothing sets fetchAsString', async () => {
      const connection = await createLiveConnection();
      const { rows } = await executeAsync(connection, SQL);
      expect(rows[0]).toEqual({ NUM: 1, BOOL: true });
    });

    it('should return the listed types as strings and leave the rest typed', async () => {
      const connection = await createLiveConnection();
      const { rows } = await executeAsync(connection, SQL, { fetchAsString: ['Number'] });
      expect(rows[0]).toEqual({ NUM: '1', BOOL: true });
    });
  });

  describe('set on the connection', () => {
    it('should apply to a query that passes no fetchAsString', async () => {
      const connection = await createLiveConnection({ fetchAsString: ['Number'] });
      const { rows } = await executeAsync(connection, SQL);
      expect(rows[0]).toEqual({ NUM: '1', BOOL: true });
    });

    it('should be replaced, not merged, by the fetchAsString a query passes', async () => {
      const connection = await createLiveConnection({ fetchAsString: ['Number'] });
      const { rows } = await executeAsync(connection, SQL, { fetchAsString: ['Boolean'] });
      expect(rows[0]).toEqual({ NUM: 1, BOOL: 'TRUE' });
    });

    it('should be turned off by an empty fetchAsString on the query', async () => {
      const connection = await createLiveConnection({ fetchAsString: ['Number'] });
      const { rows } = await executeAsync(connection, SQL, { fetchAsString: [] });
      expect(rows[0]).toEqual({ NUM: 1, BOOL: true });
    });
  });
});
