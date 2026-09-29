import { describe, it, beforeAll, afterAll, expect } from 'vitest';
import type { Connection } from '../types/sdk-types.js';
import {
  createTestConnection,
  destroyConnectionAsync,
  executeAsync,
  isRunningNewDriverWithBD,
} from './utils/index.js';

describe('Multi Statement', () => {
  let connection: Connection;

  beforeAll(async () => {
    connection = createTestConnection();
    await connection.connectAsync();
    await executeAsync(connection, 'alter session set MULTI_STATEMENT_COUNT=0');
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  it('should execute a parameterised multi-statement query and stream every sub-result', async () => {
    let cellCount = 0;
    await new Promise<void>((resolve, reject) => {
      connection.execute({
        sqlText: 'select ?; select ?,3; select ?,5,6',
        binds: [1, 2, 4],
        complete: (err, stmt) => {
          if (err) return reject(err);
          stmt
            .streamRows()
            .on('error', reject)
            .on('data', (row: Record<string, unknown>) => {
              cellCount += Object.values(row).length;
            })
            .on('end', () => {
              if ('hasNext' in stmt && stmt.hasNext()) {
                stmt.NextResult();
              } else {
                resolve();
              }
            });
        },
      });
    });

    expect(cellCount).toBe(6);
  });

  it('should expose multi-statement navigation on RowStatement', async () => {
    const { statement } = await executeAsync(connection, 'select 1');

    if (isRunningNewDriverWithBD('BD#50')) {
      expect('hasNext' in statement).toBe(true);
      expect('NextResult' in statement).toBe(true);
      if ('hasNext' in statement && 'NextResult' in statement) {
        expect(statement.hasNext()).toBe(false);
        const sqlText = statement.getSqlText();
        const queryId = statement.getQueryId();
        statement.NextResult();
        expect(statement.hasNext()).toBe(false);
        expect(statement.getSqlText()).toBe(sqlText);
        expect(statement.getQueryId()).toBe(queryId);
      }
    } else {
      expect('hasNext' in statement).toBe(false);
      expect('NextResult' in statement).toBe(false);
    }
  });

  it('should expose the per-statement SQL text while iterating with NextResult', async () => {
    const sqlText = 'select 1; select 2,3; select 4,5,6';
    const expectedSqlTexts = sqlText.split(';');

    const seenSqlTexts: string[] = [];
    await new Promise<void>((resolve, reject) => {
      connection.execute({
        sqlText,
        complete: (err, stmt) => {
          if (err) return reject(err);
          seenSqlTexts.push(stmt.getSqlText()!);
          if ('hasNext' in stmt && stmt.hasNext()) {
            stmt.NextResult();
          } else {
            resolve();
          }
        },
      });
    });

    expect(seenSqlTexts).toEqual(expectedSqlTexts);
  });
});
