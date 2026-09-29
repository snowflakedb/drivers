import { describe, it, beforeAll, afterAll, expect } from 'vitest';
import type { Connection } from '../../types/sdk-types.js';
import {
  connectionGetQueryStatusWithBD,
  createTestConnection,
  destroyConnectionAsync,
  executeAsync,
  isRunningNewDriverWithBD,
} from '../utils/index.js';

const WAIT_SECONDS = 2;
const ASYNC_WAIT_SQL = `CALL SYSTEM$WAIT(${WAIT_SECONDS}, 'SECONDS')`;
const EXPECTED_WAIT_RESULT = `waited ${WAIT_SECONDS} seconds`;

describe('Async Query Execution', () => {
  let connection: Connection;

  beforeAll(async () => {
    connection = createTestConnection();
    await connection.connectAsync();
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  it('should invoke complete with no rows and queryId on the statement', async () => {
    const { rows, statementQueryId } = await new Promise<{
      rows: unknown;
      statementQueryId: string | undefined;
    }>((resolve, reject) => {
      connection.execute({
        sqlText: ASYNC_WAIT_SQL,
        asyncExec: true,
        complete: (err, stmt, completeRows) => {
          if (err) return reject(err);
          resolve({
            rows: completeRows,
            statementQueryId: stmt.getQueryId(),
          });
        },
      });
    });

    expect(statementQueryId).toEqual(expect.any(String));
    if (isRunningNewDriverWithBD('BD#49')) {
      expect(rows).toBeUndefined();
    } else {
      expect(rows).toEqual({ queryId: statementQueryId });
    }
  });

  it('should not expose a result set on an asyncExec statement', async () => {
    const { statement } = await executeAsync(connection, 'SELECT 1 AS n', { asyncExec: true });

    expect(statement.getQueryId()).toEqual(expect.any(String));
    if (isRunningNewDriverWithBD('BD#49')) {
      expect(statement.getNumRows()).toBeUndefined();
      expect(statement.getColumns()).toBeUndefined();
      expect(statement.getColumn(0)).toBeUndefined();
      expect(statement.getColumn('N')).toBeUndefined();
    } else {
      expect(() => statement.getNumRows()).toThrow(TypeError);
      expect(() => statement.getColumns()).toThrow(TypeError);
      expect(() => statement.getColumn(0)).toThrow(TypeError);
      expect(() => statement.getColumn('N')).toThrow(TypeError);
    }
  });

  it('should allow a sync query on the same connection while an async query is running', async () => {
    const { statement: asyncStatement } = await executeAsync(
      connection,
      `CALL SYSTEM$WAIT(30, 'SECONDS')`,
      { asyncExec: true },
    );
    const queryId = asyncStatement.getQueryId()!;

    const { rows } = await executeAsync(connection, 'SELECT 1 AS n');
    expect(rows).toEqual([{ N: 1 }]);

    const status = await connectionGetQueryStatusWithBD(connection, queryId);
    expect(connection.isStillRunning(status)).toBe(true);
  });

  it('should not fail execute for invalid SQL when asyncExec is true', async () => {
    const { statement } = await executeAsync(connection, 'select * from fakeTable', {
      asyncExec: true,
    });
    expect(statement.getQueryId()).toEqual(expect.any(String));
  });

  describe('getResultsFromQueryId()', () => {
    let queryId: string;

    beforeAll(async () => {
      const asyncQuery = await executeAsync(connection, ASYNC_WAIT_SQL, { asyncExec: true });
      queryId = asyncQuery.statement.getQueryId()!;
    });

    // TODO: re-enable when running streamRows() is fixed for non completed statements
    it.skip('should return rows via stream', async () => {
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

    it('should return rows via complete callback', async () => {
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
});
