import { randomUUID } from 'node:crypto';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import type { Connection, RowStatement, SnowflakeError } from '../../types/sdk-types.js';
import { createLiveConnection, createTable, createTemporaryTable } from '../utils/fixtures.js';
import {
  destroyConnectionAsync,
  executeAsync,
  isRunningNewDriverWithBD,
  sendExecute,
} from '../utils/index.js';

describe('requestId', () => {
  let connection: Connection;

  beforeAll(async () => {
    connection = await createLiveConnection({}, false);
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  describe('RowStatement.getRequestId', () => {
    it.each([false, true])(
      'should return a UUID immediately on execute and keep it after completion (asyncExec=%s)',
      async (asyncExecValue) => {
        const { statement, completion } = sendExecute(connection, 'SELECT 1 AS n', {
          asyncExec: asyncExecValue,
        });

        const requestId = statement.getRequestId();
        expect(requestId).toBeTruthy();
        expect(statement.getQueryId()).toBeUndefined();

        await completion;

        expect(statement.getRequestId()).toBe(requestId);
        expect(statement.getQueryId()).toBeTruthy();
        expect(statement.getQueryId()).not.toBe(requestId);
      },
    );

    it.each([false, true])(
      'should round-trip a caller-supplied UUID (asyncExec=%s)',
      async (asyncExecValue) => {
        const requestId = randomUUID();
        const { statement, completion } = sendExecute(connection, 'SELECT 1 AS n', {
          requestId,
          asyncExec: asyncExecValue,
        });

        expect(statement.getRequestId()).toBe(requestId);
        await completion;
        expect(statement.getRequestId()).toBe(requestId);
      },
    );

    it('should still expose getRequestId after a failed execute', async () => {
      const { statement, completion } = sendExecute(
        connection,
        'SELECT * FROM a_table_that_does_not_exist',
      );

      const requestId = statement.getRequestId();
      expect(requestId).toBeTruthy();
      await expect(completion).rejects.toMatchObject({ error: expect.any(Error) });
      expect(statement.getRequestId()).toBe(requestId);
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

      expect(fetchedStatement.getRequestId()).toBeUndefined();
      await completion;
      expect(fetchedStatement.getRequestId()).toBeUndefined();
    });

    it('should reject a non-UUID requestId before talking to GS', async () => {
      if (isRunningNewDriverWithBD('BD#54')) {
        expect(() =>
          connection.execute({
            sqlText: 'SELECT 1 AS n',
            requestId: 'foobar',
          }),
        ).toThrow(
          expect.objectContaining({
            name: 'InvalidParameterError',
            code: 460001,
            message: 'Invalid requestId: foobar',
          } satisfies Partial<SnowflakeError>),
        );
      } else {
        const { statement, completion } = sendExecute(connection, 'SELECT 1 AS n', {
          requestId: 'foobar',
        });
        expect(statement.getRequestId()).toBe('foobar');
        await expect(completion).rejects.toMatchObject({
          error: {
            name: 'RequestFailedError',
            code: 401002,
            message: 'Request to Snowflake failed.',
          } satisfies Partial<SnowflakeError>,
        });
      }
    });
  });

  describe('execute requestId resubmit', () => {
    it('should not insert twice when the same requestId is reused on the same connection', async () => {
      const table = await createTemporaryTable(connection, 'colA string');

      const first = await executeAsync(connection, `INSERT INTO ${table} VALUES ('testValue')`);
      const requestId = first.statement.getRequestId();

      await executeAsync(connection, `INSERT INTO ${table} VALUES ('testValue')`, {
        requestId,
      });

      const { rows } = await executeAsync(connection, `SELECT * FROM ${table}`);
      expect(rows).toHaveLength(1);
    });

    it('should insert again when the same requestId is reused on a different connection', async () => {
      const secondConnection = await createLiveConnection();
      const table = await createTable(connection, 'colA string');

      const first = await executeAsync(connection, `INSERT INTO ${table} VALUES ('testValue')`);
      const requestId = first.statement.getRequestId();

      await executeAsync(secondConnection, `INSERT INTO ${table} VALUES ('testValue')`, {
        requestId,
      });

      const { rows } = await executeAsync(connection, `SELECT * FROM ${table}`);
      expect(rows).toHaveLength(2);
    });

    it('should return the cached SELECT result when the same requestId is resubmitted', async () => {
      const table = await createTemporaryTable(connection, 'colA string');

      await executeAsync(connection, `INSERT INTO ${table} VALUES ('testValue')`);
      const firstSelect = await executeAsync(connection, `SELECT * FROM ${table}`);
      expect(firstSelect.rows).toHaveLength(1);
      const firstSelectRequestId = firstSelect.statement.getRequestId();

      await executeAsync(connection, `INSERT INTO ${table} VALUES ('testValue')`);
      const { rows } = await executeAsync(connection, `SELECT * FROM ${table}`);
      expect(rows).toHaveLength(2);

      const resubmit = await executeAsync(connection, `SELECT * FROM ${table}`, {
        requestId: firstSelectRequestId,
      });
      expect(resubmit.rows).toHaveLength(1);
    });
  });
});
