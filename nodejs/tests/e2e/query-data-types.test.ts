import type { Connection } from 'snowflake-sdk-old';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import {
  createTestConnection,
  destroyConnectionAsync,
  executeAsync,
  getSnowflakeSDK,
  isRunningNewDriverWithBD,
  NOT_IMPLEMENTED_IN_NEW_DRIVER,
} from './utils/index.js';

function dateAtUtcMidnight(dateLiteral: string): Date {
  return new Date(`${dateLiteral}T00:00:00.000Z`);
}

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
      "SELECT X'ABCDEF'::BINARY as BINARY_COLUMN, NULL::BINARY as NULL_BINARY_COLUMN",
    );
    const expectedValue = Buffer.from('ABCDEF', 'hex');
    const receivedValue = rows![0].BINARY_COLUMN as Buffer;
    const binaryColumn = statement.getColumn(0);
    const nullBinaryColumn = statement.getColumn(1);
    expect(binaryColumn.getType()).toBe('binary');
    expect(binaryColumn.isBinary()).toBe(true);
    expect(nullBinaryColumn.getType()).toBe('binary');
    expect(nullBinaryColumn.isBinary()).toBe(true);
    expect(rows![0].NULL_BINARY_COLUMN).toBe(null);
    // Old-driver Buffers carry extra .toStringSf() / .getFormat() methods that make
    // vitest's .toEqual fail; the new driver returns a plain Buffer (BD#12).
    if (isRunningNewDriverWithBD('BD#12')) {
      expect(receivedValue).toEqual(expectedValue);
    } else {
      expect(receivedValue.equals(expectedValue)).toBe(true);
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

  describe('DATE', () => {
    it('casts date values to appropriate type', async () => {
      const { statement, rows } = await executeAsync(
        connection,
        `SELECT
          '2024-01-15'::DATE AS DATE_2024_01_15,
          '1970-01-01'::DATE AS EPOCH_DATE,
          '1999-12-31'::DATE AS DATE_1999_12_31`,
      );
      for (const columnName of ['DATE_2024_01_15', 'EPOCH_DATE', 'DATE_1999_12_31']) {
        const column = statement.getColumn(columnName);
        expect(column.getType()).toBe('date');
        expect(column.isDate()).toBe(true);
        expect(rows![0][columnName]).toBeInstanceOf(Date);
      }
    });

    it('selects date literals', async () => {
      const { rows } = await executeAsync(
        connection,
        `SELECT '2024-01-15'::DATE, '1970-01-01'::DATE, '1999-12-31'::DATE`,
      );
      expect(Object.values(rows![0])).toEqual([
        dateAtUtcMidnight('2024-01-15'),
        dateAtUtcMidnight('1970-01-01'),
        dateAtUtcMidnight('1999-12-31'),
      ]);
    });

    it('selects epoch and pre-epoch dates', async () => {
      const { rows } = await executeAsync(
        connection,
        `SELECT '1970-01-01'::DATE, '1969-12-31'::DATE, '1900-01-01'::DATE`,
      );
      expect(Object.values(rows![0])).toEqual([
        dateAtUtcMidnight('1970-01-01'),
        dateAtUtcMidnight('1969-12-31'),
        dateAtUtcMidnight('1900-01-01'),
      ]);
    });

    it('selects historical and boundary dates', async () => {
      const { rows } = await executeAsync(
        connection,
        // CUTOVER_DATE (1582-10-15) is the Julian-to-Gregorian cutover date, so
        // it pins the decoder to a proleptic Gregorian calendar.
        `SELECT '0001-01-01'::DATE, '1582-10-15'::DATE, '9999-12-31'::DATE`,
      );
      expect(Object.values(rows![0])).toEqual([
        dateAtUtcMidnight('0001-01-01'),
        dateAtUtcMidnight('1582-10-15'),
        dateAtUtcMidnight('9999-12-31'),
      ]);
    });

    it('handles NULL values for date', async () => {
      const { rows } = await executeAsync(
        connection,
        `SELECT NULL::DATE, '2024-01-15'::DATE, NULL::DATE as null_column2`,
      );
      expect(Object.values(rows![0])).toEqual([null, dateAtUtcMidnight('2024-01-15'), null]);
    });
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

  describe.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)('TIMESTAMP', () => {
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
