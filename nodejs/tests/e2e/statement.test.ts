import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import type { Connection, RowStatement } from '../types/sdk-types.js';
import { createLiveConnection } from './utils/fixtures.js';
import {
  sendExecute,
  destroyConnectionAsync,
  executeAsync,
  NOT_IMPLEMENTED_IN_NEW_DRIVER,
} from './utils/index.js';

describe('RowStatement.getSqlText', () => {
  let connection: Connection;

  beforeAll(async () => {
    connection = await createLiveConnection({}, false);
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  it('should keep bind placeholders in the SQL text instead of inlining bound values', async () => {
    const sqlText = 'SELECT ?';
    const { statement, completion } = sendExecute(connection, sqlText, { binds: [1] });

    expect(statement.getSqlText()).toBe(sqlText);
    await completion;
    expect(statement.getSqlText()).toBe(sqlText);
  });

  it('should return the submitted SQL before and after failed execution', async () => {
    const sqlText = 'select;';
    const { statement, completion } = sendExecute(connection, sqlText);

    expect(statement.getSqlText()).toBe(sqlText);
    await expect(completion).rejects.toMatchObject({ error: expect.any(Error) });
    expect(statement.getSqlText()).toBe(sqlText);
  });

  it('should return undefined for a statement created by fetchResult', async () => {
    const { statement: executedStatement } = await executeAsync(connection, 'SELECT 1');
    const queryId = executedStatement.getQueryId();
    expect(queryId).toBeDefined();

    let fetchedStatement!: RowStatement;
    const completion = new Promise<RowStatement>((resolve, reject) => {
      fetchedStatement = connection.fetchResult({
        queryId: queryId!,
        complete: (error, completedStatement) => {
          if (error) {
            reject(error);
          } else {
            resolve(completedStatement);
          }
        },
      });
    });

    expect(fetchedStatement.getSqlText()).toBeUndefined();
    await completion;
    expect(fetchedStatement.getSqlText()).toBeUndefined();
  });

  it.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)(
    'should return undefined for a statement created by getResultsFromQueryId',
    async () => {
      const { statement: executedStatement } = await executeAsync(connection, 'SELECT 1');
      const queryId = executedStatement.getQueryId();
      expect(queryId).toBeDefined();

      const resultsStatement = await connection.getResultsFromQueryId({ queryId: queryId! });
      expect(resultsStatement.getSqlText()).toBeUndefined();
    },
  );
});
