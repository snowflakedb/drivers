import type { Connection, QueryStatus, SnowflakeError } from 'snowflake-sdk';
import { ErrorCode } from 'snowflake-sdk';
import { describe, it, beforeAll, afterAll, expect } from 'vitest';
import { createConnection, connectAsync, destroyAsync, sleepAsync, executeAsync } from './utils';

const WAIT_SECONDS = 2;
const ASYNC_WAIT_SQL = `CALL SYSTEM$WAIT(${WAIT_SECONDS}, 'SECONDS')`;
const EXPECTED_WAIT_RESULT = `waited ${WAIT_SECONDS} seconds`;

const NON_EXISTENT_QUERY_ID = '12345678-1234-4123-A123-123456789012';

describe('Async Query Execution', () => {
  let connection: Connection;

  beforeAll(async () => {
    connection = createConnection();
    await connectAsync(connection);
  });

  afterAll(async () => {
    await destroyAsync(connection);
  });

  describe('getQueryStatus()', () => {
    it('returns RUNNING for pending async query', async () => {
      // TODO: snowflake-sdk types getQueryStatus() as Promise<string> but
      // isStillRunning() takes the QueryStatus literal union. Drop the cast
      // once the public types narrow getQueryStatus() to QueryStatus.
      const { statement } = await executeAsync(connection, ASYNC_WAIT_SQL, { asyncExec: true });
      const status = (await connection.getQueryStatus(statement.getQueryId())) as QueryStatus;
      expect(status).toBe('RUNNING');
      expect(connection.isStillRunning(status)).toBe(true);
    });

    it('returns NO_QUERY_DATA for a non-existent query id', async () => {
      expect(await connection.getQueryStatus(NON_EXISTENT_QUERY_ID)).toBe('NO_QUERY_DATA');
    });

    it('rejects with ERR_GET_RESPONSE_QUERY_INVALID_UUID for a malformed query id', async () => {
      await expect(connection.getQueryStatus('fakeQueryId')).rejects.toMatchObject({
        code: ErrorCode.ERR_GET_RESPONSE_QUERY_INVALID_UUID,
      } satisfies Partial<SnowflakeError>);
    });
  });

  describe('getResultsFromQueryId()', () => {
    let queryId: string;

    beforeAll(async () => {
      const asyncQuery = await executeAsync(connection, ASYNC_WAIT_SQL, { asyncExec: true });
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

    it('rejects with ERR_GET_RESPONSE_QUERY_INVALID_UUID for a malformed query id', async () => {
      await expect(
        connection.getResultsFromQueryId({ queryId: 'fakeQueryId' }),
      ).rejects.toMatchObject({
        code: ErrorCode.ERR_GET_RESPONSE_QUERY_INVALID_UUID,
      } satisfies Partial<SnowflakeError>);
    });

    it('rejects with ERR_GET_RESULTS_QUERY_ID_NO_DATA for a valid but non-existent query id', async () => {
      await expect(
        connection.getResultsFromQueryId({ queryId: NON_EXISTENT_QUERY_ID }),
      ).rejects.toMatchObject({
        code: ErrorCode.ERR_GET_RESULTS_QUERY_ID_NO_DATA,
      } satisfies Partial<SnowflakeError>);
    });
  });

  it('surfaces an error for a query that failed server-side via getQueryStatusThrowIfError and getResultsFromQueryId', async () => {
    const failedQuery = await executeAsync(connection, 'select * from fakeTable', {
      asyncExec: true,
    });
    const queryId = failedQuery.statement.getQueryId();
    while (connection.isStillRunning((await connection.getQueryStatus(queryId)) as QueryStatus)) {
      await sleepAsync(250);
    }

    const status = await connection.getQueryStatus(queryId);
    expect(status).toBe('FAILED_WITH_ERROR');
    // TODO: snowflake-sdk index.d.ts declares isAnError() with no args, but the
    // implementation takes a status string. Drop the cast once the public types catch up.
    expect((connection.isAnError as (s: string) => boolean)(status)).toBe(true);

    await expect(connection.getQueryStatusThrowIfError(queryId)).rejects.toMatchObject({
      name: 'OperationFailedError',
    });
    await expect(connection.getResultsFromQueryId({ queryId })).rejects.toMatchObject({
      name: 'OperationFailedError',
    });
  });
});
