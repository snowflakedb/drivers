import { describe, it, expect } from 'vitest';
import type { Connection } from '../types/sdk-types.js';
import { createLiveConnection } from './utils/fixtures.js';
import { executeAsync } from './utils/index.js';

const ROW_COUNTS = [37, 14, 92, 71, 63];

const selectRows = (rowCount: number) => `select true from table(generator(rowcount=>${rowCount}))`;

describe('Concurrent Execution', () => {
  describe('Regular result', () => {
    async function getRowCount(connection: Connection, rowCount: number): Promise<number> {
      const { rows } = await executeAsync(connection, selectRows(rowCount));
      return rows.length;
    }

    it('should run many concurrent select queries on a single connection', async () => {
      const connection = await createLiveConnection();
      const rowCounts = await Promise.all(
        ROW_COUNTS.map((rowCount) => getRowCount(connection, rowCount)),
      );
      expect(rowCounts).toEqual(ROW_COUNTS);
    });

    it('should run concurrent select queries on independent connections', async () => {
      const connections = await Promise.all(ROW_COUNTS.map(() => createLiveConnection()));
      const rowCounts = await Promise.all(
        connections.map((connection, i) => getRowCount(connection, ROW_COUNTS[i])),
      );
      expect(rowCounts).toEqual(ROW_COUNTS);
    });
  });

  describe('Streamed result', () => {
    async function streamRowCount(connection: Connection, rowCount: number): Promise<number> {
      const { statement } = await executeAsync(connection, selectRows(rowCount), {
        streamResult: true,
      });
      const rows = await statement.streamRows().toArray();
      return rows.length;
    }

    it('should run many concurrent select queries on a single connection', async () => {
      const connection = await createLiveConnection();
      const rowCounts = await Promise.all(
        ROW_COUNTS.map((rowCount) => streamRowCount(connection, rowCount)),
      );
      expect(rowCounts).toEqual(ROW_COUNTS);
    });

    it('should run concurrent select queries on independent connections', async () => {
      const connections = await Promise.all(ROW_COUNTS.map(() => createLiveConnection()));
      const rowCounts = await Promise.all(
        connections.map((connection, i) => streamRowCount(connection, ROW_COUNTS[i])),
      );
      expect(rowCounts).toEqual(ROW_COUNTS);
    });
  });
});
