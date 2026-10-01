import { describe, it, beforeAll, afterAll, beforeEach, expect, onTestFinished } from 'vitest';
import type { Connection, ConnectionOptions, Pool, SnowflakeError } from '../types/sdk-types.js';
import {
  baseConnectionOptions,
  executeAsync,
  isRunningNewDriverWithBD,
  snowflake,
  TEST_CONNECTION_OPTIONS,
} from './utils/index.js';
import { heartbeatSuccess, proxyAllTo, WiremockServer } from './utils/wiremock/index.js';

function createTestPool(
  poolOptions: { max: number; min: number; testOnBorrow?: boolean },
  connectionOptions: Partial<ConnectionOptions> = {},
): Pool<Connection> {
  const pool = snowflake.createPool(
    { ...TEST_CONNECTION_OPTIONS, ...connectionOptions },
    poolOptions,
  );
  onTestFinished(async () => {
    await pool.drain();
    await pool.clear();
  });
  return pool;
}

async function acquireFrom(pool: Pool<Connection>): Promise<Connection> {
  const connection = await pool.acquire();
  onTestFinished(async () => {
    if (pool.isBorrowedResource(connection)) {
      await pool.release(connection);
    }
  });
  return connection;
}

describe('Connection Pool', () => {
  let pool: Pool<Connection>;

  beforeAll(() => {
    pool = snowflake.createPool({ ...TEST_CONNECTION_OPTIONS }, { max: 10, min: 0 });
  });

  afterAll(async () => {
    await pool.drain();
    await pool.clear();
  });

  describe('use()', () => {
    it('should run concurrent callbacks, each with distinct results', async () => {
      const expectedValues = [2837, 6104, 1592, 8471, 3963];
      const results = await Promise.all(
        expectedValues.map((n) =>
          pool.use(async (connection) => {
            const { rows } = await executeAsync(connection, `select ${n} as N`);
            return rows[0].N;
          }),
        ),
      );
      expect(results).toEqual(expectedValues);
    });

    it('should propagate errors thrown inside the callback', async () => {
      await expect(
        pool.use((connection) =>
          executeAsync(connection, 'select * from a_table_that_does_not_exist'),
        ),
      ).rejects.toMatchObject({
        error: {
          name: 'OperationFailedError',
          code: '002003',
          sqlState: '42S02',
          message: expect.stringContaining('does not exist or not authorized'),
        } satisfies Partial<SnowflakeError>,
      });
    });

    it('should reuse an idle connection on a second use', async () => {
      const reusePool = createTestPool({ max: 5, min: 0 });
      expect(reusePool.size).toBe(0);

      await reusePool.use(async (connection) => {
        expect(connection.isUp()).toBe(true);
        expect(reusePool.size).toBe(1);
        expect(reusePool.available).toBe(0);
      });
      expect(reusePool.size).toBe(1);
      expect(reusePool.available).toBe(1);

      await reusePool.use(async (connection) => {
        expect(connection.isUp()).toBe(true);
        expect(reusePool.size).toBe(1);
        expect(reusePool.available).toBe(0);
      });
      expect(reusePool.size).toBe(1);
      expect(reusePool.available).toBe(1);
    });
  });

  describe('acquire() + release()', () => {
    it('should borrow a usable connection and a second acquire after release works too', async () => {
      const conn1 = await acquireFrom(pool);
      const { rows: rows1 } = await executeAsync(conn1, 'select 1 as N');
      expect(rows1).toEqual([{ N: 1 }]);
      await pool.release(conn1);

      const conn2 = await acquireFrom(pool);
      const { rows: rows2 } = await executeAsync(conn2, 'select 2 as N');
      expect(rows2).toEqual([{ N: 2 }]);
      await pool.release(conn2);
    });
  });

  describe('acquire() + destroy()', () => {
    it('should evict a borrowed connection and return a usable one on the next acquire', async () => {
      const evicted = await acquireFrom(pool);
      await pool.destroy(evicted);

      const replacement = await acquireFrom(pool);
      expect(replacement).not.toBe(evicted);
      await pool.release(replacement);
    });
  });

  describe('pool sizing', () => {
    it('should create min connections at construction', () => {
      const sizedPool = createTestPool({ max: 10, min: 1 });
      expect(sizedPool.max).toBe(10);
      expect(sizedPool.min).toBe(1);
      expect(sizedPool.size).toBe(1);
    });

    it('should clamp min to max when min is greater than max', () => {
      const sizedPool = createTestPool({ max: 1, min: 2 });
      expect(sizedPool.max).toBe(1);
      expect(sizedPool.min).toBe(1);
      expect(sizedPool.size).toBe(1);
    });

    it('should not grow beyond max when acquire is called more times than max', async () => {
      const sizedPool = createTestPool({ max: 1, min: 0 });
      const first = await acquireFrom(sizedPool);
      expect(sizedPool.size).toBe(1);

      const secondPromise = acquireFrom(sizedPool);
      expect(sizedPool.size).toBe(1);
      expect(sizedPool.pending).toBe(1);

      await sizedPool.release(first);
      const second = await secondPromise;
      await sizedPool.release(second);
    });
  });

  describe('factory create failure', () => {
    it('should reject acquire when connecting with invalid credentials', async () => {
      const failingPool = createTestPool({ max: 1, min: 0 }, { username: 'incorrect-username' });
      await expect(failingPool.acquire()).rejects.toMatchObject({
        name: 'Error',
        message: expect.stringContaining('JWT token is invalid'),
        ...(isRunningNewDriverWithBD('BD#55') ? { code: '390144', sqlState: '28000' } : {}),
      } satisfies Partial<SnowflakeError>);
    });

    it('should reject use when connecting with invalid credentials', async () => {
      const failingPool = createTestPool({ max: 1, min: 0 }, { username: 'incorrect-username' });
      await expect(failingPool.use(async () => undefined)).rejects.toMatchObject({
        name: 'Error',
        message: expect.stringContaining('JWT token is invalid'),
        ...(isRunningNewDriverWithBD('BD#55') ? { code: '390144', sqlState: '28000' } : {}),
      } satisfies Partial<SnowflakeError>);
    });
  });

  describe('with wiremock', () => {
    let wiremock: WiremockServer;

    beforeAll(async () => {
      wiremock = await WiremockServer.spawn();
    });

    afterAll(async () => {
      await wiremock.destroy();
    });

    beforeEach(async () => {
      await wiremock.reset();
      await wiremock.stub(proxyAllTo(`https://${baseConnectionOptions.host}`));
      await wiremock.stub(heartbeatSuccess());
    });

    it('should send a heartbeat on each acquire', async () => {
      const heartbeatPool = createTestPool(
        { max: 1, min: 0, testOnBorrow: true },
        wiremock.connectionOptions,
      );

      const first = await acquireFrom(heartbeatPool);
      expect(await wiremock.findRequests('/session/heartbeat.*')).toHaveLength(1);
      await heartbeatPool.release(first);

      await wiremock.clearRequests();

      const second = await acquireFrom(heartbeatPool);
      expect(await wiremock.findRequests('/session/heartbeat.*')).toHaveLength(1);
      await heartbeatPool.release(second);
    });
  });
});
