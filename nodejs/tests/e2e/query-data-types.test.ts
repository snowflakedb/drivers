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

  // NOTE: BINARY_OUTPUT_FORMAT (HEX or BASE64) does not affect results.
  // The server always returns HEX and it is always converted to Buffer.
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

  describe('TIMESTAMP', () => {
    it('returns TIMESTAMP_LTZ as Date with zone offset', async () => {
      const { statement, rows } = await executeAsync(
        connection,
        "SELECT to_timestamp_ltz('Thu, 21 Jan 2016 06:32:44 -0800') as LTZ_COLUMN",
      );
      const column = statement.getColumn(0);
      const selectedValue = rows![0].LTZ_COLUMN as Date;
      expect(column.isTimestamp()).toBe(true);
      expect(column.getType()).toBe('timestamp_ltz');
      expect(selectedValue).toBeInstanceOf(Date);
      expect(selectedValue.toJSON()).toBe('2016-01-21 06:32:44.000 -0800');
    });

    it('returns TIMESTAMP_TZ as Date with zone offset', async () => {
      const { statement, rows } = await executeAsync(
        connection,
        "SELECT to_timestamp_tz('Thu, 21 Jan 2016 06:32:44 -0800') as TZ_COLUMN",
      );
      const column = statement.getColumn(0);
      const selectedValue = rows![0].TZ_COLUMN as Date;
      expect(column.isTimestamp()).toBe(true);
      expect(column.getType()).toBe('timestamp_tz');
      expect(selectedValue).toBeInstanceOf(Date);
      expect(selectedValue.toJSON()).toBe('2016-01-21 06:32:44.000 -0800');
    });

    it('returns TIMESTAMP_NTZ as Date with no zone offset', async () => {
      const { statement, rows } = await executeAsync(
        connection,
        "SELECT to_timestamp_ntz('Thu, 21 Jan 2016 06:32:44 -0800') as NTZ_COLUMN",
      );
      const column = statement.getColumn(0);
      const selectedValue = rows![0].NTZ_COLUMN as Date;
      expect(column.isTimestamp()).toBe(true);
      expect(column.getType()).toBe('timestamp_ntz');
      expect(selectedValue).toBeInstanceOf(Date);
      expect(selectedValue.toJSON()).toBe('2016-01-21 06:32:44.000');
    });
  });
});
