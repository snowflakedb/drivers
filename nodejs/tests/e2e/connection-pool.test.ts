import type { Connection, Pool } from 'snowflake-sdk-old';
import { describe, it, beforeAll, afterAll, expect } from 'vitest';
import {
  executeAsync,
  getSnowflakeSDK,
  NOT_IMPLEMENTED_IN_NEW_DRIVER,
  TEST_CONNECTION_OPTIONS,
} from './utils/index.js';

async function executeSelect(connection: Connection, numValue: number): Promise<number> {
  const { rows } = await executeAsync(connection, `select ${numValue} as N`);
  return (rows![0] as { N: number }).N;
}

describe.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)('Connection Pool', () => {
  const snowflake = getSnowflakeSDK();
  let pool: Pool<Connection>;

  beforeAll(() => {
    pool = snowflake.createPool(TEST_CONNECTION_OPTIONS, { max: 10, min: 0, testOnBorrow: true });
  });

  afterAll(async () => {
    await pool.drain();
    await pool.clear();
  });

  describe('use()', () => {
    it('runs concurrent callbacks, each with distinct results', async () => {
      const expectedValues = [2837, 6104, 1592, 8471, 3963];
      const results = await Promise.all(
        expectedValues.map((n) => pool.use((connection) => executeSelect(connection, n))),
      );
      expect(results).toEqual(expectedValues);
    });

    it('propagates errors thrown inside the callback', async () => {
      await expect(
        pool.use((connection) => executeAsync(connection, 'select from non_existent_table')),
      ).rejects.toMatchObject({
        error: expect.objectContaining({
          name: 'OperationFailedError',
          code: '001003',
        }),
      });
    });
  });

  describe('acquire() + release()', () => {
    it('borrows a usable connection and a second acquire() after release works too', async () => {
      const conn1 = await pool.acquire();
      try {
        expect(await executeSelect(conn1, 1)).toBe(1);
      } finally {
        await pool.release(conn1);
      }

      const conn2 = await pool.acquire();
      try {
        expect(await executeSelect(conn2, 2)).toBe(2);
      } finally {
        await pool.release(conn2);
      }
    });
  });

  describe('acquire() + destroy()', () => {
    it('evicts a borrowed connection and a follow-up acquire() still returns a usable one', async () => {
      const evicted = await pool.acquire();
      await pool.destroy(evicted);

      const replacement = await pool.acquire();
      try {
        expect(replacement).not.toBe(evicted);
      } finally {
        await pool.release(replacement);
      }
    });
  });
});
