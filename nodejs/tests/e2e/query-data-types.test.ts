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

  // NOTE: BINARY_OUTPUT_FORMAT (HEX or BASE64) does not affect results,
  // since the server always returns HEX, which is always converted to Buffer client-side.
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

  it('returns BOOLEAN as Boolean', async () => {
    const { statement, rows } = await executeAsync(
      connection,
      'SELECT TRUE::BOOLEAN as TRUE_COLUMN, FALSE::BOOLEAN as FALSE_COLUMN',
    );
    const trueColumn = statement.getColumn(0);
    const falseColumn = statement.getColumn(1);
    expect(trueColumn.getType()).toBe('boolean');
    expect(trueColumn.isBoolean()).toBe(true);
    expect(falseColumn.getType()).toBe('boolean');
    expect(falseColumn.isBoolean()).toBe(true);
    expect(rows![0].TRUE_COLUMN).toBe(true);
    expect(rows![0].FALSE_COLUMN).toBe(false);
  });

  it('returns DECFLOAT as String', async () => {
    const { statement, rows } = await executeAsync(
      connection,
      "SELECT '-9.8765432099999998623226732747455716901e-250'::DECFLOAT as DECFLOAT_COLUMN",
    );
    // NO is* methods cover decfloat
    expect(statement.getColumn(0).getType()).toBe('decfloat');
    expect(rows![0].DECFLOAT_COLUMN).toBe('-9.8765432099999998623226732747455716901e-250');
  });

  // NOTE: DATE_OUTPUT_FORMAT does not affect results, we always convert to Date
  it('returns DATE as Date', async () => {
    const { statement, rows } = await executeAsync(
      connection,
      "SELECT '2026-01-01'::DATE as DATE_COLUMN",
    );
    const column = statement.getColumn(0);
    const selectedValue = rows![0].DATE_COLUMN as Date;
    expect(column.getType()).toBe('date');
    expect(column.isDate()).toBe(true);
    expect(selectedValue).toBeInstanceOf(Date);
    expect(selectedValue.toISOString()).toEqual('2026-01-01T00:00:00.000Z');
  });

  describe('TIME', () => {
    it('returns HH:MM:SS string by default', async () => {
      const { statement, rows } = await executeAsync(
        connection,
        "SELECT '12:34:56.789789789'::TIME as TIME_COLUMN",
      );
      const column = statement.getColumn(0);
      expect(column.getType()).toBe('time');
      expect(column.isTime()).toBe(true);
      expect(rows![0].TIME_COLUMN).toBe('12:34:56');
    });

    it('returns HH:MM:SS.SSSSSSSSS string when TIME_OUTPUT_FORMAT is set', async () => {
      try {
        await executeAsync(connection, "ALTER SESSION SET TIME_OUTPUT_FORMAT = 'HH:MI:SS.FF9'");
        const { statement, rows } = await executeAsync(
          connection,
          "SELECT '12:34:56.789789789'::TIME as TIME_COLUMN",
        );
        const column = statement.getColumn(0);
        expect(column.getType()).toBe('time');
        expect(column.isTime()).toBe(true);
        expect(rows![0].TIME_COLUMN).toBe('12:34:56.789789789');
      } finally {
        await executeAsync(connection, 'ALTER SESSION UNSET TIME_OUTPUT_FORMAT');
      }
    });
  });

  it('returns OBJECT/MAP as Object', async () => {
    const { statement, rows } = await executeAsync(
      connection,
      "SELECT OBJECT_CONSTRUCT('key', 'value') as OBJECT_COLUMN, {'key': 'value'}::MAP(VARCHAR, VARCHAR) as MAP_COLUMN",
    );
    const expectedValue = { key: 'value' };
    const objectColumn = statement.getColumn(0);
    const mapColumn = statement.getColumn(1);
    expect(objectColumn.getType()).toBe('object');
    expect(mapColumn.getType()).toBe('object');
    // Bug in old driver: isObject() returns false because server doesn't return fieldsMetadata
    if (isRunningForOldDriver()) {
      expect(objectColumn.isObject()).toBe(false);
      expect(mapColumn.isObject()).toBe(false);
    } else {
      expect(objectColumn.isObject()).toBe(true);
      expect(mapColumn.isObject()).toBe(true);
    }
    expect(rows![0].OBJECT_COLUMN).toEqual(expectedValue);
    expect(rows![0].MAP_COLUMN).toEqual(expectedValue);
  });

  it('returns ARRAY as Array', async () => {
    const { statement, rows } = await executeAsync(
      connection,
      'SELECT ARRAY_CONSTRUCT(1, 2, 3) as ARRAY_COLUMN',
    );
    const column = statement.getColumn(0);
    expect(column.getType()).toBe('array');
    // Bug in old driver: isArray() returns false because server doesn't return fieldsMetadata
    if (isRunningForOldDriver()) {
      expect(column.isArray()).toBe(false);
    } else {
      expect(column.isArray()).toBe(true);
    }
    expect(rows![0].ARRAY_COLUMN).toEqual([1, 2, 3]);
  });
});
