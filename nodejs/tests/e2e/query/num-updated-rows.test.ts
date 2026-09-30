import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import type { Connection } from '../../types/sdk-types.js';
import {
  createLiveConnection,
  createTempDir,
  createTemporaryStage,
  createTemporaryTable,
} from '../utils/fixtures.js';
import {
  destroyConnectionAsync,
  executeAsync,
  isRunningNewDriverWithBD,
  sendExecute,
} from '../utils/index.js';

describe('RowStatement.getNumUpdatedRows', () => {
  let connection: Connection;

  beforeAll(async () => {
    connection = await createLiveConnection({}, false);
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  it('should return undefined before the statement completes', async () => {
    const { statement, completion } = sendExecute(connection, `CALL SYSTEM$WAIT(1, 'SECONDS')`);

    expect(statement.getNumUpdatedRows()).toBeUndefined();
    await completion;
    expect(statement.getNumUpdatedRows()).toBe(-1);
  });

  it('should return -1 for CREATE TABLE (DDL)', async () => {
    const tableName = await createTemporaryTable(connection, 'c1 NUMBER');
    const { statement } = await executeAsync(
      connection,
      `CREATE OR REPLACE TEMPORARY TABLE ${tableName} (c1 NUMBER)`,
    );

    expect(statement.getNumUpdatedRows()).toBe(-1);
  });

  it('should return -1 for SELECT (DQL)', async () => {
    const { statement } = await executeAsync(connection, 'SELECT 1');

    expect(statement.getNumUpdatedRows()).toBe(-1);
  });

  it('should report insert, update, and delete counts (DML)', async () => {
    const table1 = await createTemporaryTable(connection, 'c1 NUMBER, c2 NUMBER');
    const table2 = await createTemporaryTable(connection, 'c3 NUMBER, c4 NUMBER');

    const { statement: insertTable1 } = await executeAsync(
      connection,
      `INSERT INTO ${table1} VALUES (1,10),(2,10),(3,10),(4,10),(5,10),(6,10),(7,10),(8,10)`,
    );
    expect(insertTable1.getNumUpdatedRows()).toBe(8);

    const { statement: insertTable2 } = await executeAsync(
      connection,
      `INSERT INTO ${table2} VALUES (1,5),(2,5),(3,5),(1,5)`,
    );
    expect(insertTable2.getNumUpdatedRows()).toBe(4);

    const { statement: updateTable1 } = await executeAsync(
      connection,
      `UPDATE ${table1} SET c2 = c4 FROM ${table2} WHERE ${table1}.c1 = ${table2}.c3`,
    );
    expect(updateTable1.getNumUpdatedRows()).toBe(4);

    const { statement: deleteTable1 } = await executeAsync(connection, `DELETE FROM ${table1}`);
    expect(deleteTable1.getNumUpdatedRows()).toBe(8);
  });

  it('should report the merge insert, update, and delete sum', async () => {
    const table1 = await createTemporaryTable(connection, 'k NUMBER, v NUMBER');
    const table2 = await createTemporaryTable(connection, 'k NUMBER, v NUMBER');

    await executeAsync(
      connection,
      `INSERT INTO ${table1} VALUES (0, 10), (1, 11), (5, 15), (6, 16)`,
    );
    await executeAsync(
      connection,
      `INSERT INTO ${table2} VALUES (0, 20), (1, 21), (2, 22), (3, 23), (4, 24)`,
    );

    const { statement } = await executeAsync(
      connection,
      `MERGE INTO ${table1} USING ${table2} ON ${table1}.k = ${table2}.k ` +
        `WHEN MATCHED AND ${table1}.k = 0 THEN UPDATE SET v = ${table2}.v * 100 ` +
        `WHEN MATCHED AND ${table1}.k = 1 THEN DELETE ` +
        `WHEN NOT MATCHED THEN INSERT VALUES (k, v)`,
    );
    expect(statement.getNumUpdatedRows()).toBe(5);
  });

  it('should report the multi-table insert sum', async () => {
    const source = await createTemporaryTable(connection, 'k NUMBER, v NUMBER');
    const table1 = await createTemporaryTable(connection, 'k NUMBER, v NUMBER');
    const table2 = await createTemporaryTable(connection, 'k NUMBER, v NUMBER');

    await executeAsync(connection, `INSERT INTO ${source} VALUES (0, 100), (1, 101), (2, 102)`);

    const { statement } = await executeAsync(
      connection,
      `INSERT ALL INTO ${table1} INTO ${table2} SELECT * FROM ${source}`,
    );
    expect(statement.getNumUpdatedRows()).toBe(6);
  });

  it('should report COPY rows_loaded instead of -1', async () => {
    const table = await createTemporaryTable(connection, 'c1 NUMBER, c2 NUMBER');
    const stage = await createTemporaryStage(connection);
    const dir = createTempDir();
    dir.writeFile('data.csv', '1,10\n2,20\n3,30\n');

    await executeAsync(connection, `PUT ${dir.fileUrl('data.csv')} @${stage} AUTO_COMPRESS=FALSE`);

    const { statement } = await executeAsync(
      connection,
      `COPY INTO ${table} FROM @${stage} FILE_FORMAT = (TYPE = CSV)`,
    );
    expect(statement.getNumUpdatedRows()).toBe(isRunningNewDriverWithBD('BD#53') ? 3 : -1);
  });

  it('should return -1 for UNLOAD', async () => {
    const table = await createTemporaryTable(connection, 'c1 NUMBER, c2 NUMBER');
    const stage = await createTemporaryStage(connection);
    await executeAsync(connection, `INSERT INTO ${table} VALUES (1, 10), (2, 20), (3, 30)`);

    const { statement } = await executeAsync(connection, `COPY INTO @${stage} FROM ${table}`);
    expect(statement.getNumUpdatedRows()).toBe(-1);
  });

  it('should return -1 for PUT and GET', async () => {
    const stage = await createTemporaryStage(connection);
    const uploadDir = createTempDir();
    const downloadDir = createTempDir();
    uploadDir.writeFile('data.csv', '1,10\n');

    const { statement: putStatement } = await executeAsync(
      connection,
      `PUT ${uploadDir.fileUrl('data.csv')} @${stage} AUTO_COMPRESS=FALSE`,
    );
    expect(putStatement.getNumUpdatedRows()).toBe(-1);

    const { statement: getStatement } = await executeAsync(
      connection,
      `GET @${stage}/data.csv ${downloadDir.fileUrl()}/`,
    );
    expect(getStatement.getNumUpdatedRows()).toBe(-1);
  });
});
