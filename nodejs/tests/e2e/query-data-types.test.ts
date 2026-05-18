import { Connection } from 'snowflake-sdk';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import {
  createTestConnection,
  destroyConnectionAsync,
  executeAsync,
  getSnowflakeSDK,
  isRunningForOldDriver,
} from './utils';

describe('Query returning data types', () => {
  const snowflake = getSnowflakeSDK();
  let connection: Connection;

  beforeAll(async () => {
    connection = createTestConnection(snowflake);
    await connection.connectAsync();
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  it('returns STRING-like types as String', async () => {
    const { statement, rows } = await executeAsync(
      connection,
      "SELECT 'a'::VARCHAR as A, 'b'::CHAR as B, 'c'::STRING as C, 'd'::TEXT as D",
    );
    const expectedValues: Record<string, string> = {
      A: 'a',
      B: 'b',
      C: 'c',
      D: 'd',
    };
    for (const colName of Object.keys(expectedValues)) {
      const column = statement.getColumn(colName);
      expect(column.getType()).toBe('text');
      expect(column.isString()).toBe(true);
      expect(rows![0][colName]).toBe(expectedValues[colName]);
    }
  });

  it('returns BINARY as Buffer', async () => {
    const { statement, rows } = await executeAsync(
      connection,
      "SELECT X'ABCDEF'::BINARY as BINARY_COLUMN",
    );
    const expectedValue = Buffer.from('ABCDEF', 'hex');
    const receivedValue = rows![0].BINARY_COLUMN as Buffer;
    const column = statement.getColumn(0);
    expect(column.getType()).toBe('binary');
    expect(column.isBinary()).toBe(true);
    // In old driver, returned Buffer has extra methods .toStringSf() and .getFormat())
    // that cause .toEqual to fail in vitest. New driver omits these monkey patches (see BCR log).
    if (isRunningForOldDriver()) {
      expect(receivedValue.equals(expectedValue)).toBe(true);
    } else {
      expect(receivedValue).toEqual(expectedValue);
    }
  });

  it('returns NULL as null', async () => {
    const { statement, rows } = await executeAsync(connection, 'SELECT NULL::TEXT as NULL_COLUMN');
    const column = statement.getColumn(0);
    expect(column.getType()).toBe('text');
    expect(rows![0].NULL_COLUMN).toBe(null);
  });

  it('returns BOOLEAN as boolean', async () => {
    const { statement, rows } = await executeAsync(
      connection,
      'SELECT TRUE::BOOLEAN as TRUE_COLUMN, FALSE::BOOLEAN as FALSE_COLUMN',
    );
    for (const column of statement.getColumns()!) {
      expect(column.getType()).toBe('boolean');
      expect(column.isBoolean()).toBe(true);
    }
    expect(rows![0].TRUE_COLUMN).toBe(true);
    expect(rows![0].FALSE_COLUMN).toBe(false);
  });
});
