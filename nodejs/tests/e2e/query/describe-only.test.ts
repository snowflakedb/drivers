import fs from 'node:fs';
import path from 'node:path';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import type { Connection, SnowflakeError } from '../../types/sdk-types.js';
import { sharedTestDataDir, toFileUrl } from '../utils/files.js';
import { createLiveConnection, createTempDir, createTemporaryStage } from '../utils/fixtures.js';
import {
  cancelStatementAsync,
  destroyConnectionAsync,
  executeAsync,
  expectColumnsNames,
  getResultFromQueryIdForTest,
  getStatementColumn,
  isRunningNewDriverWithBD,
  NOT_IMPLEMENTED_IN_NEW_DRIVER,
  randomizeName,
} from '../utils/index.js';

const DESCRIBE_SQL = "SELECT 1.0::NUMBER(30,2) AS C1, 't3' AS C2";
const DESCRIBE_COLUMN_NAMES = ['C1', 'C2'];
const PUT_GET_TEST_FILE = path.join(sharedTestDataDir(), 'compression', 'test_data.csv');

describe('describeOnly', () => {
  let connection: Connection;

  beforeAll(async () => {
    connection = await createLiveConnection({}, false);
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  it('should return column metadata and empty rows when describeOnly is true', async () => {
    const { statement, rows } = await executeAsync(connection, DESCRIBE_SQL, {
      describeOnly: true,
    });

    expect(rows).toEqual([]);
    expect(statement.getSqlText()).toBe(DESCRIBE_SQL);
    expect(statement.getNumRows()).toBe(0);
    expect(statement.getQueryId()).toBeDefined();
    expectColumnsNames(statement, DESCRIBE_COLUMN_NAMES);
    const column = getStatementColumn(statement, 0);
    expect({
      name: column.getName(),
      index: column.getIndex(),
      id: column.getId(),
      nullable: column.isNullable(),
      scale: column.getScale(),
      precision: column.getPrecision(),
      type: column.getType(),
    }).toMatchObject({
      name: 'C1',
      index: 0,
      id: 0,
      nullable: false,
      scale: 2,
      precision: 30,
      type: 'fixed',
    });
  });

  it('should return rows when describeOnly is false', async () => {
    const { statement, rows } = await executeAsync(connection, DESCRIBE_SQL, {
      describeOnly: false,
    });

    expect(rows).toHaveLength(1);
    expectColumnsNames(statement, DESCRIBE_COLUMN_NAMES);
    expect(Object.keys(rows[0])).toEqual(DESCRIBE_COLUMN_NAMES);
  });

  it('should never returns metadata when running asyncExec with describeOnly', async () => {
    const { statement, rows } = await executeAsync(connection, DESCRIBE_SQL, {
      describeOnly: true,
      asyncExec: true,
    });
    const queryId = statement.getQueryId();

    // Unfinished statement doesn't contain any metadata
    expect(queryId).toEqual(expect.any(String));
    if (isRunningNewDriverWithBD('BD#49')) {
      expect(rows).toBeUndefined();
      expect(statement.getNumRows()).toBeUndefined();
      expect(statement.getColumns()).toBeUndefined();
    } else {
      expect(rows).toEqual({ queryId });
      expect(() => statement.getNumRows()).toThrow(TypeError);
      expect(() => statement.getColumns()).toThrow(TypeError);
    }

    // Finished describeOnly statement is never retrieved from the server
    const result = getResultFromQueryIdForTest(connection, { queryId: queryId! });
    await expect(result).rejects.toMatchObject({
      name: 'ClientError',
      code: 460002,
      message: isRunningNewDriverWithBD('BD#43')
        ? `Cannot retrieve data. No information returned from server for query ${queryId}`
        : `Cannot retrieve data. No information returned from server for query ${queryId} NO_QUERY_DATA`,
    } satisfies Partial<SnowflakeError>);
  });

  it('should reject invalid SQL when describeOnly is true', async () => {
    const tableName = randomizeName('TEST_DESCRIBE_ONLY');
    const execution = executeAsync(connection, `select * from ${tableName}`, {
      describeOnly: true,
    });

    await expect(execution).rejects.toMatchObject({
      error: {
        name: 'OperationFailedError',
        code: '002003',
        sqlState: '42S02',
        message: expect.stringContaining('does not exist or not authorized'),
      } satisfies Partial<SnowflakeError>,
    });
  });

  it('should describe a bound query without returning rows', async () => {
    const { statement, rows } = await executeAsync(connection, 'SELECT ? AS C1', {
      describeOnly: true,
      binds: [1],
    });

    expect(rows).toEqual([]);
    expectColumnsNames(statement, ['C1']);
  });

  it('should reject PUT when describeOnly is true', async () => {
    const stage = await createTemporaryStage(connection);

    const execution = executeAsync(connection, `PUT ${toFileUrl(PUT_GET_TEST_FILE)} @${stage}`, {
      describeOnly: true,
    });

    await expect(execution).rejects.toMatchObject({
      error: {
        name: 'OperationFailedError',
        code: '000007',
        sqlState: '0A000',
        message: 'Statement provided can not be prepared.',
      } satisfies Partial<SnowflakeError>,
    });

    const { rows } = await executeAsync(connection, `LS @${stage}`);
    expect(rows).toEqual([]);
  });

  it('should reject GET when describeOnly is true', async () => {
    // put file to stage
    const stage = await createTemporaryStage(connection);
    await executeAsync(
      connection,
      `PUT ${toFileUrl(PUT_GET_TEST_FILE)} @${stage} AUTO_COMPRESS=TRUE OVERWRITE=TRUE`,
    );

    // get file from stage with describeOnly = true
    const downloadDir = createTempDir();
    const execution = executeAsync(
      connection,
      `GET @${stage}/test_data.csv ${downloadDir.fileUrl()}/`,
      {
        describeOnly: true,
      },
    );

    await expect(execution).rejects.toMatchObject({
      error: {
        name: 'OperationFailedError',
        code: '000007',
        sqlState: '0A000',
        message: 'Statement provided can not be prepared.',
      } satisfies Partial<SnowflakeError>,
    });

    expect(fs.readdirSync(downloadDir.path)).toEqual([]);
  });

  it.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)(
    'should throw when cancelling a completed describeOnly statement',
    async () => {
      const { statement } = await executeAsync(connection, DESCRIBE_SQL, { describeOnly: true });

      const cancellation = cancelStatementAsync(statement);

      await expect(cancellation).rejects.toMatchObject({
        code: '390413',
        message: expect.stringContaining('does not exist or not authorized'),
      } satisfies Partial<SnowflakeError>);
    },
  );

  // describeOnly on a multi-statement query is not supported by snowflake server.
  // the result doesn't contain child query IDs, so the bridge throws an error.
  it('should describe a multi-statement query without returning rows', async () => {
    const multiConnection = await createLiveConnection();
    await executeAsync(multiConnection, 'alter session set MULTI_STATEMENT_COUNT=0');

    const sqlText = 'select 1 as A; select 2 as B';
    const execution = executeAsync(multiConnection, sqlText, {
      describeOnly: true,
    });

    if (isRunningNewDriverWithBD('BD#56')) {
      await expect(execution).rejects.toMatchObject({
        error: {
          message: 'multi-statement result contained no child query IDs',
        },
      });
    } else {
      const { statement, rows } = await execution;
      expect(rows).toEqual([]);
      expect(statement.getNumRows()).toBe(0);
      expect(statement.getSqlText()).toBe(sqlText);
      expectColumnsNames(statement, ['multiple statement execution']);
      expect('hasNext' in statement).toBe(false);
    }
  });
});
