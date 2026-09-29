import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import type { Connection, RowStatement } from '../types/sdk-types.js';
import { createLiveConnection } from './utils/fixtures.js';
import {
  sendExecute,
  destroyConnectionAsync,
  executeAsync,
  expectColumnsNames,
  getStatementColumn,
} from './utils/index.js';

describe('RowStatement', () => {
  let connection: Connection;

  beforeAll(async () => {
    connection = await createLiveConnection({}, false);
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  describe('getSqlText', () => {
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

    it('should return undefined for a statement created by getResultsFromQueryId', async () => {
      const { statement: executedStatement } = await executeAsync(connection, 'SELECT 1');
      const queryId = executedStatement.getQueryId();
      expect(queryId).toBeDefined();

      const resultsStatement = await connection.getResultsFromQueryId({ queryId: queryId! });
      expect(resultsStatement.getSqlText()).toBeUndefined();
    });
  });

  describe('result-set getters', () => {
    it('should return from result-set getters only after a successful execute', async () => {
      const { statement, completion } = sendExecute(connection, 'SELECT 1 AS n');

      expect(statement.getNumRows()).toBeUndefined();
      expect(statement.getColumns()).toBeUndefined();
      expect(statement.getColumn(0)).toBeUndefined();
      expect(statement.getColumn('N')).toBeUndefined();

      await completion;

      expect(statement.getNumRows()).toBe(1);
      expectColumnsNames(statement as RowStatement, ['N']);
      expect(getStatementColumn(statement as RowStatement, 0).getName()).toBe('N');
    });

    it('should return undefined from result-set getters after a failed execute', async () => {
      const { statement, completion } = sendExecute(
        connection,
        'select * from a_table_that_does_not_exist',
      );

      expect(statement.getNumRows()).toBeUndefined();
      expect(statement.getColumns()).toBeUndefined();
      expect(statement.getColumn(0)).toBeUndefined();
      expect(statement.getColumn('COL')).toBeUndefined();

      await expect(completion).rejects.toMatchObject({ error: expect.any(Error) });
      expect(statement.getNumRows()).toBeUndefined();
      expect(statement.getColumns()).toBeUndefined();
      expect(statement.getColumn(0)).toBeUndefined();
      expect(statement.getColumn('COL')).toBeUndefined();
    });
  });
});
