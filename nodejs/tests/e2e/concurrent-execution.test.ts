import type { RowStatement } from 'snowflake-sdk';
import { describe, it, expect } from 'vitest';
import {
  createTestConnection,
  destroyConnectionAsync,
  executeAsync,
  getSnowflakeSDK,
} from './utils';

const selectRows = (rowCount: number) => `select true from table(generator(rowcount=>${rowCount}))`;

function streamRowCount(stmt: RowStatement): Promise<number> {
  return new Promise((resolve, reject) => {
    const stream = stmt.streamRows();
    let rowCount = 0;
    stream.on('readable', () => {
      while (stream.read() !== null) {
        rowCount++;
      }
    });
    stream.on('error', reject);
    stream.on('end', () => resolve(rowCount));
  });
}

describe('Concurrent Execution', () => {
  const snowflake = getSnowflakeSDK();

  it('runs many concurrent select queries on a single connection', async () => {
    const expectedRowCounts = [2837, 6104, 1592, 8471, 3963];
    const connection = createTestConnection(snowflake);
    await connection.connectAsync();

    try {
      const rowCounts = await Promise.all(
        expectedRowCounts.map(async (expected) => {
          const { statement } = await executeAsync(connection, selectRows(expected));
          return streamRowCount(statement as RowStatement);
        }),
      );
      expect(rowCounts).toEqual(expectedRowCounts);
    } finally {
      await destroyConnectionAsync(connection);
    }
  });

  it('runs concurrent select queries on independent connections', async () => {
    const expectedRowCounts = [4218, 1736, 7905, 2649, 5380];
    const connections = expectedRowCounts.map(() => createTestConnection(snowflake));

    try {
      await Promise.all(connections.map((c) => c.connectAsync()));
      const rowCounts = await Promise.all(
        connections.map(async (c, i) => {
          const { statement } = await executeAsync(c, selectRows(expectedRowCounts[i]));
          return streamRowCount(statement as RowStatement);
        }),
      );
      expect(rowCounts).toEqual(expectedRowCounts);
    } finally {
      await Promise.all(connections.map((c) => destroyConnectionAsync(c).catch(() => undefined)));
    }
  });
});
