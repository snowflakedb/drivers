import { describe, it, beforeAll, afterAll, expect } from 'vitest';
import type { Connection, QueryStatus } from '../../types/sdk-types.js';
import {
  createTestConnection,
  destroyConnectionAsync,
  sleepAsync,
  executeAsync,
  NOT_IMPLEMENTED_IN_NEW_DRIVER,
} from '../utils/index.js';

const WAIT_SECONDS = 2;
const ASYNC_WAIT_SQL = `CALL SYSTEM$WAIT(${WAIT_SECONDS}, 'SECONDS')`;
const EXPECTED_WAIT_RESULT = `waited ${WAIT_SECONDS} seconds`;

describe.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)('Async Query Execution', () => {
  let connection: Connection;

  beforeAll(async () => {
    connection = createTestConnection();
    await connection.connectAsync();
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  describe('getResultsFromQueryId()', () => {
    let queryId: string;

    beforeAll(async () => {
      const asyncQuery = await executeAsync(connection, ASYNC_WAIT_SQL, { asyncExec: true });
      // @ts-ignore NOT_IMPLEMENTED_IN_NEW_DRIVER
      queryId = asyncQuery.statement.getQueryId();
    });

    it('returns rows via stream', async () => {
      const resultQuery = await connection.getResultsFromQueryId({ queryId });
      const rows: Record<string, unknown>[] = [];
      await new Promise<void>((resolve, reject) => {
        resultQuery
          .streamRows()
          .on('error', reject)
          .on('data', (row: Record<string, unknown>) => rows.push(row))
          .on('end', () => resolve());
      });
      expect(rows).toEqual([{ SYSTEM$WAIT: EXPECTED_WAIT_RESULT }]);
      expect(await connection.getQueryStatus(queryId)).toBe('SUCCESS');
    });

    it('returns rows via complete callback', async () => {
      const rows = await new Promise<unknown[]>((resolve, reject) => {
        connection.getResultsFromQueryId({
          queryId,
          complete: (err, _stmt, fetchedRows) => {
            if (err) return reject(err);
            resolve(fetchedRows ?? []);
          },
        });
      });
      expect(rows).toEqual([{ SYSTEM$WAIT: EXPECTED_WAIT_RESULT }]);
      expect(await connection.getQueryStatus(queryId)).toBe('SUCCESS');
    });
  });

  it('surfaces an error for a query that failed server-side via getQueryStatusThrowIfError and getResultsFromQueryId', async () => {
    const failedQuery = await executeAsync(connection, 'select * from fakeTable', {
      asyncExec: true,
    });
    const queryId = failedQuery.statement.getQueryId();
    // Cast: upstream `getQueryStatus` is typed `Promise<string>`, but the
    // server only returns `QueryStatus` literals and `isStillRunning` requires one.
    // TODO: we'll have BD for this
    // @ts-ignore NOT_IMPLEMENTED_IN_NEW_DRIVER
    while (connection.isStillRunning((await connection.getQueryStatus(queryId)) as QueryStatus)) {
      await sleepAsync(250);
    }

    // TODO: we'll have BD for this
    // @ts-ignore NOT_IMPLEMENTED_IN_NEW_DRIVER
    const status = (await connection.getQueryStatus(queryId)) as QueryStatus;
    expect(status).toBe('FAILED_WITH_ERROR');
    // Cast: upstream `isAnError` is typed as zero-arg; the runtime takes a status.
    // TODO: we'll have BD for this
    expect((connection.isAnError as (status: QueryStatus) => boolean)(status)).toBe(true);

    // @ts-ignore NOT_IMPLEMENTED_IN_NEW_DRIVER
    await expect(connection.getQueryStatusThrowIfError(queryId)).rejects.toMatchObject({
      name: 'OperationFailedError',
    });
    // @ts-ignore NOT_IMPLEMENTED_IN_NEW_DRIVER
    await expect(connection.getResultsFromQueryId({ queryId })).rejects.toMatchObject({
      name: 'OperationFailedError',
    });
  });
});
