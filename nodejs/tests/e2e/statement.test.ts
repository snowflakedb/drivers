import type { RowStatement as NewRowStatement } from 'snowflake-sdk';
import { afterAll, beforeAll, describe, expect, it, onTestFinished } from 'vitest';
import type { Connection, RowStatement, SessionState } from '../types/sdk-types.js';
import { createLiveConnection } from './utils/fixtures.js';
import {
  sendExecute,
  destroyConnectionAsync,
  executeAsync,
  expectColumnsNames,
  getStatementColumn,
  isRunningNewDriverWithBD,
  randomizeName,
} from './utils/index.js';

function getSessionState(statement: RowStatement): SessionState | undefined {
  if (isRunningNewDriverWithBD('BD#57')) {
    return (statement as NewRowStatement).getSessionState();
  }
  return statement.getSessionState() as SessionState | undefined;
}

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
      expect(statement.getNumUpdatedRows()).toBeUndefined();
      expect(statement.getColumns()).toBeUndefined();
      expect(statement.getColumn(0)).toBeUndefined();
      expect(statement.getColumn('N')).toBeUndefined();
      expect(statement.getSessionState()).toBeUndefined();

      await completion;

      expect(statement.getNumRows()).toBe(1);
      expect(statement.getNumUpdatedRows()).toBe(-1);
      expectColumnsNames(statement as RowStatement, ['N']);
      expect(getStatementColumn(statement as RowStatement, 0).getName()).toBe('N');
      expect(statement.getSessionState()).toBeDefined();
    });

    it('should return undefined from result-set getters after a failed execute', async () => {
      const { statement, completion } = sendExecute(
        connection,
        'select * from a_table_that_does_not_exist',
      );

      expect(statement.getNumRows()).toBeUndefined();
      expect(statement.getNumUpdatedRows()).toBeUndefined();
      expect(statement.getColumns()).toBeUndefined();
      expect(statement.getColumn(0)).toBeUndefined();
      expect(statement.getColumn('COL')).toBeUndefined();
      expect(statement.getSessionState()).toBeUndefined();

      await expect(completion).rejects.toMatchObject({ error: expect.any(Error) });
      expect(statement.getNumRows()).toBeUndefined();
      expect(statement.getNumUpdatedRows()).toBeUndefined();
      expect(statement.getColumns()).toBeUndefined();
      expect(statement.getColumn(0)).toBeUndefined();
      expect(statement.getColumn('COL')).toBeUndefined();
      expect(statement.getSessionState()).toBeUndefined();
    });
  });

  describe('getSessionState', () => {
    it('should match CURRENT_DATABASE, CURRENT_SCHEMA, CURRENT_WAREHOUSE, and CURRENT_ROLE after a query', async () => {
      const { statement, rows } = await executeAsync(
        connection,
        'SELECT CURRENT_DATABASE() AS DATABASE_NAME, CURRENT_SCHEMA() AS SCHEMA_NAME, CURRENT_WAREHOUSE() AS WAREHOUSE_NAME, CURRENT_ROLE() AS ROLE_NAME',
      );

      const sessionState = getSessionState(statement);
      expect(sessionState).toBeDefined();
      expect(sessionState!.getCurrentDatabase()).toBe(rows[0].DATABASE_NAME);
      expect(sessionState!.getCurrentSchema()).toBe(rows[0].SCHEMA_NAME);
      expect(sessionState!.getCurrentWarehouse()).toBe(rows[0].WAREHOUSE_NAME);
      expect(sessionState!.getCurrentRole()).toBe(rows[0].ROLE_NAME);
      if (isRunningNewDriverWithBD('BD#58')) {
        expect(sessionState).not.toHaveProperty('getCurrentDatabaseProvider');
      } else {
        expect(sessionState).toHaveProperty('getCurrentDatabaseProvider');
      }
    });

    it('should report the new schema from the USE SCHEMA statement and keep the earlier statement snapshot', async () => {
      const { statement: beforeStatement, rows: beforeRows } = await executeAsync(
        connection,
        'SELECT CURRENT_SCHEMA() AS SCHEMA_NAME',
      );
      const originalSchema = String(beforeRows[0].SCHEMA_NAME);
      const schema = randomizeName('NODEJS_SESSION_STATE_SCHEMA_');
      await executeAsync(connection, `CREATE SCHEMA ${schema}`);
      onTestFinished(async () => {
        await executeAsync(connection, `USE SCHEMA ${originalSchema}`);
        await executeAsync(connection, `DROP SCHEMA IF EXISTS ${schema}`);
      });

      const { statement: useStatement } = await executeAsync(connection, `USE SCHEMA ${schema}`);
      expect(getSessionState(useStatement)?.getCurrentSchema()?.toUpperCase()).toBe(
        schema.toUpperCase(),
      );
      expect(getSessionState(beforeStatement)?.getCurrentSchema()?.toUpperCase()).toBe(
        originalSchema.toUpperCase(),
      );
    });

    it('should report the new database from the USE DATABASE statement and keep the earlier statement snapshot', async () => {
      const { statement: beforeStatement, rows: beforeRows } = await executeAsync(
        connection,
        'SELECT CURRENT_DATABASE() AS DATABASE_NAME',
      );
      const originalDatabase = String(beforeRows[0].DATABASE_NAME);
      const database = randomizeName('NODEJS_SESSION_STATE_DB_');
      await executeAsync(connection, `CREATE DATABASE ${database}`);
      onTestFinished(async () => {
        await executeAsync(connection, `USE DATABASE ${originalDatabase}`);
        await executeAsync(connection, `DROP DATABASE IF EXISTS ${database}`);
      });

      const { statement: useStatement } = await executeAsync(
        connection,
        `USE DATABASE ${database}`,
      );
      expect(getSessionState(useStatement)?.getCurrentDatabase()?.toUpperCase()).toBe(
        database.toUpperCase(),
      );
      expect(getSessionState(beforeStatement)?.getCurrentDatabase()?.toUpperCase()).toBe(
        originalDatabase.toUpperCase(),
      );
    });

    it('should keep each concurrent USE SCHEMA snapshot on its own statement', async () => {
      const { rows } = await executeAsync(connection, 'SELECT CURRENT_SCHEMA() AS SCHEMA_NAME');
      const originalSchema = String(rows[0].SCHEMA_NAME);
      const schemaA = randomizeName('NODEJS_SESSION_STATE_A_');
      const schemaB = randomizeName('NODEJS_SESSION_STATE_B_');
      onTestFinished(async () => {
        await executeAsync(connection, `USE SCHEMA ${originalSchema}`);
        await executeAsync(connection, `DROP SCHEMA IF EXISTS ${schemaA}`);
        await executeAsync(connection, `DROP SCHEMA IF EXISTS ${schemaB}`);
      });
      await executeAsync(connection, `CREATE SCHEMA ${schemaA}`);
      await executeAsync(connection, `CREATE SCHEMA ${schemaB}`);

      const [resultA, resultB] = await Promise.all([
        executeAsync(connection, `USE SCHEMA ${schemaA}`),
        executeAsync(connection, `USE SCHEMA ${schemaB}`),
      ]);
      expect(getSessionState(resultA.statement)?.getCurrentSchema()?.toUpperCase()).toBe(
        schemaA.toUpperCase(),
      );
      expect(getSessionState(resultB.statement)?.getCurrentSchema()?.toUpperCase()).toBe(
        schemaB.toUpperCase(),
      );
    });
  });
});
