import type { Connection, FileAndStageBindStatement, RowStatement } from 'snowflake-sdk';
import { describe, it, beforeAll, afterAll, expect } from 'vitest';
import { createConnection, connectAsync, destroyAsync, executeAsync } from './utils';

describe('Multi Statement', () => {
  let connection: Connection;

  beforeAll(async () => {
    connection = createConnection();
    await connectAsync(connection);
    await executeAsync(connection, 'alter session set MULTI_STATEMENT_COUNT=0');
  });

  afterAll(async () => {
    await destroyAsync(connection);
  });

  it('executes a parameterised multi-statement query and streams rows from every sub-result', async () => {
    let cellCount = 0;
    await new Promise<void>((resolve, reject) => {
      connection.execute({
        sqlText: 'select ?; select ?,3; select ?,5,6',
        binds: [1, 2, 4],
        complete: (err, stmt) => {
          if (err) return reject(err);
          // TODO: We need better TypeScript support for multi-statements
          const multiStmt = stmt as FileAndStageBindStatement;
          const stream = (stmt as RowStatement).streamRows();
          stream.on('error', reject);
          stream.on('data', (row: Record<string, unknown>) => {
            cellCount += Object.values(row).length;
            if (multiStmt.hasNext()) {
              multiStmt.NextResult();
            } else {
              resolve();
            }
          });
        },
      });
    });

    expect(cellCount).toBe(6);
  });

  it('exposes the per-statement SQL text while iterating with NextResult', async () => {
    const sqlText = 'select 1; select 2,3; select 4,5,6';
    const expectedSqlTexts = sqlText.split(';');

    const seenSqlTexts: string[] = [];
    await new Promise<void>((resolve, reject) => {
      connection.execute({
        sqlText,
        complete: (err, stmt) => {
          if (err) return reject(err);
          seenSqlTexts.push(stmt.getSqlText());
          // TODO: We need better TypeScript support for multi-statements
          const multiStmt = stmt as FileAndStageBindStatement;
          if (multiStmt.hasNext()) {
            multiStmt.NextResult();
          } else {
            resolve();
          }
        },
      });
    });

    expect(seenSqlTexts).toEqual(expectedSqlTexts);
  });
});
