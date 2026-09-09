import { describe, it, expect } from 'vitest';
import type { RowStatement } from '../types/sdk-types.js';
import { createLiveConnection } from './utils/fixtures.js';
import { executeAsync, NOT_IMPLEMENTED_IN_NEW_DRIVER } from './utils/index.js';

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

describe.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)('Concurrent Execution', () => {
  it('runs many concurrent select queries on a single connection', async () => {
    const expectedRowCounts = [2837, 6104, 1592, 8471, 3963];
    const connection = await createLiveConnection();
    const rowCounts = await Promise.all(
      expectedRowCounts.map(async (expected) => {
        const { statement } = await executeAsync(connection, selectRows(expected));
        return streamRowCount(statement as RowStatement);
      }),
    );
    expect(rowCounts).toEqual(expectedRowCounts);
  });

  it('runs concurrent select queries on independent connections', async () => {
    const expectedRowCounts = [4218, 1736, 7905, 2649, 5380];
    const connections = await Promise.all(expectedRowCounts.map(() => createLiveConnection()));
    const rowCounts = await Promise.all(
      connections.map(async (connection, i) => {
        const { statement } = await executeAsync(connection, selectRows(expectedRowCounts[i]));
        return streamRowCount(statement as RowStatement);
      }),
    );
    expect(rowCounts).toEqual(expectedRowCounts);
  });
});
