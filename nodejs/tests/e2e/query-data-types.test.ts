import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import type { Connection } from '../types/sdk-types.js';
import {
  createTestConnection,
  destroyConnectionAsync,
  executeAsync,
  getStatementColumn,
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
      const column = getStatementColumn(statement, colName);
      expect(column.getType()).toBe('text');
      expect(column.isString()).toBe(true);
      expect(rows![0][colName]).toBe(expectedValues[colName]);
    }
  });

  // A TEXT cell is already a string, so rendering its NULL is the only thing the
  // String token has left to do.
  it("returns TEXT unchanged, and its NULL as the string 'NULL', when fetchAsString is set", async () => {
    const { rows } = await executeAsync(connection, "SELECT 'a'::TEXT, NULL::TEXT", {
      fetchAsString: ['String'],
    });
    expect(Object.values(rows![0])).toEqual(['a', 'NULL']);
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
    const binaryColumn = getStatementColumn(statement, 0);
    const nullBinaryColumn = getStatementColumn(statement, 1);
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

  it('returns BINARY as upper-case hex when fetchAsString is set', async () => {
    const { rows } = await executeAsync(connection, "SELECT X'ABCDEF'::BINARY, NULL::BINARY", {
      fetchAsString: ['Buffer'],
    });
    expect(Object.values(rows![0])).toEqual(['ABCDEF', 'NULL']);
  });

  it.todo('returns BINARY as base64 when fetchAsString is set and BINARY_OUTPUT_FORMAT is BASE64');

  it('returns NULL as null', async () => {
    const { statement, rows } = await executeAsync(connection, 'SELECT NULL::TEXT as NULL_COLUMN');
    const column = getStatementColumn(statement, 0);
    expect(column.getType()).toBe('text');
    expect(rows![0].NULL_COLUMN).toBe(null);
  });

  it('returns BOOLEAN as Boolean', async () => {
    const { statement, rows } = await executeAsync(
      connection,
      'SELECT TRUE::BOOLEAN as TRUE_COLUMN, FALSE::BOOLEAN as FALSE_COLUMN',
    );
    const trueColumn = getStatementColumn(statement, 0);
    const falseColumn = getStatementColumn(statement, 1);
    expect(trueColumn.getType()).toBe('boolean');
    expect(trueColumn.isBoolean()).toBe(true);
    expect(falseColumn.getType()).toBe('boolean');
    expect(falseColumn.isBoolean()).toBe(true);
    expect(rows![0].TRUE_COLUMN).toBe(true);
    expect(rows![0].FALSE_COLUMN).toBe(false);
  });

  it('returns BOOLEAN as an upper-case string when fetchAsString is set', async () => {
    const { rows } = await executeAsync(
      connection,
      'SELECT TRUE::BOOLEAN, FALSE::BOOLEAN, NULL::BOOLEAN',
      { fetchAsString: ['Boolean'] },
    );
    expect(Object.values(rows![0])).toEqual(['TRUE', 'FALSE', 'NULL']);
  });

  // NOTE: DATE_OUTPUT_FORMAT does not affect results, we always convert to Date
  it('returns DATE as Date', async () => {
    const { statement, rows } = await executeAsync(
      connection,
      "SELECT '2026-01-01'::DATE as DATE_COLUMN",
    );
    const column = getStatementColumn(statement, 0);
    const selectedValue = rows![0].DATE_COLUMN as Date;
    expect(column.getType()).toBe('date');
    expect(column.isDate()).toBe(true);
    expect(selectedValue).toBeInstanceOf(Date);
    expect(selectedValue.toISOString()).toEqual('2026-01-01T00:00:00.000Z');
  });

  // Gherkin-tracked coverage for tests/definitions/shared/types/date.feature (@nodejs_e2e).
  // Only these 5 scenarios have a faithful nodejs equivalent; the rest of date.feature
  // (table operations, parameter binding, large result sets) has no nodejs coverage yet.
  describe('DATE', () => {
    it('should cast date values to appropriate type', async () => {
      // Given Snowflake client is logged in
      const query = `SELECT
        '2024-01-15'::DATE AS DATE_2024_01_15,
        '1970-01-01'::DATE AS EPOCH_DATE,
        '1999-12-31'::DATE AS DATE_1999_12_31`;

      // When Query "SELECT '2024-01-15'::DATE, '1970-01-01'::DATE, '1999-12-31'::DATE" is executed
      const { statement, rows } = await executeAsync(connection, query);

      const columnNames = ['DATE_2024_01_15', 'EPOCH_DATE', 'DATE_1999_12_31'];
      for (const columnName of columnNames) {
        const column = getStatementColumn(statement, columnName);
        // Then All values should be returned as DATE type
        expect(column.getType()).toBe('date');
        expect(column.isDate()).toBe(true);
        // And No precision loss should occur
        expect(rows![0][columnName]).toBeInstanceOf(Date);
      }
    });

    it('should select date literals', async () => {
      // Given Snowflake client is logged in
      const query = `SELECT '2024-01-15'::DATE, '1970-01-01'::DATE, '1999-12-31'::DATE`;

      // When Query "SELECT '2024-01-15'::DATE, '1970-01-01'::DATE, '1999-12-31'::DATE" is executed
      const { rows } = await executeAsync(connection, query);

      // Then Result should contain dates [2024-01-15, 1970-01-01, 1999-12-31]
      expect(Object.values(rows![0])).toEqual([
        dateAtUtcMidnight('2024-01-15'),
        dateAtUtcMidnight('1970-01-01'),
        dateAtUtcMidnight('1999-12-31'),
      ]);
    });

    it('should select epoch and pre-epoch dates', async () => {
      // Given Snowflake client is logged in
      const query = `SELECT '1970-01-01'::DATE, '1969-12-31'::DATE, '1900-01-01'::DATE`;

      // When Query "SELECT '1970-01-01'::DATE, '1969-12-31'::DATE, '1900-01-01'::DATE" is executed
      const { rows } = await executeAsync(connection, query);

      // Then Result should contain dates [1970-01-01, 1969-12-31, 1900-01-01]
      expect(Object.values(rows![0])).toEqual([
        dateAtUtcMidnight('1970-01-01'),
        dateAtUtcMidnight('1969-12-31'),
        dateAtUtcMidnight('1900-01-01'),
      ]);
    });

    it('should select historical and boundary dates', async () => {
      // Given Snowflake client is logged in
      //
      // 1582-10-15 is the Julian-to-Gregorian cutover date, so it pins the decoder to a
      // proleptic Gregorian calendar.
      const query = `SELECT '0001-01-01'::DATE, '1582-10-15'::DATE, '9999-12-31'::DATE`;

      // When Query "SELECT '0001-01-01'::DATE, '1582-10-15'::DATE, '9999-12-31'::DATE" is executed
      const { rows } = await executeAsync(connection, query);

      // Then Result should contain dates [0001-01-01, 1582-10-15, 9999-12-31]
      expect(Object.values(rows![0])).toEqual([
        dateAtUtcMidnight('0001-01-01'),
        dateAtUtcMidnight('1582-10-15'),
        dateAtUtcMidnight('9999-12-31'),
      ]);
    });

    it('should handle NULL values for date', async () => {
      // Given Snowflake client is logged in
      const query = `SELECT NULL::DATE, '2024-01-15'::DATE, NULL::DATE as null_column2`;

      // When Query "SELECT NULL::DATE, '2024-01-15'::DATE, NULL::DATE" is executed
      const { rows } = await executeAsync(connection, query);

      // Then Result should contain [NULL, 2024-01-15, NULL]
      expect(Object.values(rows![0])).toEqual([null, dateAtUtcMidnight('2024-01-15'), null]);
    });

    it("renders dates as DATE_OUTPUT_FORMAT's default when fetchAsString is set", async () => {
      const { rows } = await executeAsync(
        connection,
        `SELECT '2024-01-15'::DATE, '1900-01-01'::DATE, '9999-12-31'::DATE, NULL::DATE`,
        { fetchAsString: ['Date'] },
      );
      expect(Object.values(rows![0])).toEqual(['2024-01-15', '1900-01-01', '9999-12-31', 'NULL']);
    });

    it.todo('honors a non-default DATE_OUTPUT_FORMAT when fetchAsString is set');
  });

  describe('TIME', () => {
    it('returns HH:MM:SS string by default', async () => {
      const { statement, rows } = await executeAsync(
        connection,
        "SELECT '12:34:56.789789789'::TIME as TIME_COLUMN",
      );
      const column = getStatementColumn(statement, 0);
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
        const column = getStatementColumn(statement, 0);
        expect(column.getType()).toBe('time');
        expect(column.isTime()).toBe(true);
        expect(rows![0].TIME_COLUMN).toBe('12:34:56.789789789');
      } finally {
        await executeAsync(connection, 'ALTER SESSION UNSET TIME_OUTPUT_FORMAT');
      }
    });

    // Gherkin-tracked coverage for tests/definitions/shared/types/time.feature (@nodejs_e2e).
    // Only these 2 of time.feature's scenarios have a nodejs equivalent — the rest need
    // fractional-second output (nodejs's default TIME_OUTPUT_FORMAT truncates to whole
    // seconds), CREATE TABLE test infrastructure, or parameter binding, none of which exist yet.
    it('should cast time values to appropriate type', async () => {
      // Given Snowflake client is logged in
      const query = `SELECT '10:30:00'::TIME AS T1, '00:00:00'::TIME AS T2, '23:59:59'::TIME AS T3`;

      // When Query "SELECT '10:30:00'::TIME, '00:00:00'::TIME, '23:59:59'::TIME" is executed
      const { statement } = await executeAsync(connection, query);

      // Then All values should be returned as appropriate type
      for (const columnName of ['T1', 'T2', 'T3']) {
        const column = getStatementColumn(statement, columnName);
        expect(column.getType()).toBe('time');
        expect(column.isTime()).toBe(true);
      }
    });

    it('selects time literals', async () => {
      const { rows } = await executeAsync(
        connection,
        `SELECT '00:00:00'::TIME, '10:30:00'::TIME, '14:45:30'::TIME, '23:59:59'::TIME`,
      );
      expect(Object.values(rows![0])).toEqual(['00:00:00', '10:30:00', '14:45:30', '23:59:59']);
    });

    // Supersedes the old 2-column NULL test: this is the Gherkin-exact scenario (3 columns,
    // matching time.feature verbatim) and is a superset of what the old test checked.
    it('should handle NULL values for time', async () => {
      // Given Snowflake client is logged in
      const query = `SELECT '10:30:00'::TIME, NULL::TIME, '23:59:59'::TIME`;

      // When Query "SELECT '10:30:00'::TIME, NULL::TIME, '23:59:59'::TIME" is executed
      const { rows } = await executeAsync(connection, query);

      // Then Result should contain [10:30:00, NULL, 23:59:59]
      if (isRunningNewDriverWithBD('BD#14')) {
        expect(Object.values(rows![0])).toEqual(['10:30:00', null, '23:59:59']);
      } else {
        expect(Object.values(rows![0])).toEqual(['10:30:00', 'NULL', '23:59:59']);
      }
    });

    // TODO: pass TIME_OUTPUT_FORMAT as statement params once execute()
    // forwards them into SessionParams — avoids an ALTER SESSION round trip
    // and keeps the format local to the query (connection-safe under concurrency).
    describe('HH24:MI:SS.FF9 format', () => {
      beforeAll(async () => {
        await executeAsync(connection, "ALTER SESSION SET TIME_OUTPUT_FORMAT = 'HH24:MI:SS.FF9'");
      });

      afterAll(async () => {
        await executeAsync(connection, 'ALTER SESSION UNSET TIME_OUTPUT_FORMAT');
      });

      it('preserves nanosecond precision for time', async () => {
        const { rows } = await executeAsync(connection, "SELECT '10:30:00.123456789'::TIME");
        expect(Object.values(rows![0])).toEqual(['10:30:00.123456789']);
      });

      it.each([
        { scale: 0, expected: '10:30:00', oldDriverExpected: '10:30:00.000000000' },
        { scale: 3, expected: '10:30:00.123', oldDriverExpected: '10:30:00.123000000' },
        { scale: 6, expected: '10:30:00.123456', oldDriverExpected: '10:30:00.123456000' },
      ])(
        'handles time precision at scale $scale',
        async ({ scale, expected, oldDriverExpected }) => {
          const { rows } = await executeAsync(
            connection,
            `SELECT '10:30:00.123456789'::TIME(${scale})`,
          );
          if (isRunningNewDriverWithBD('BD#16')) {
            expect(Object.values(rows![0])).toEqual([expected]);
          } else {
            expect(Object.values(rows![0])).toEqual([oldDriverExpected]);
          }
        },
      );
    });

    describe('HH12:MI:SS.FF9 format', () => {
      beforeAll(async () => {
        await executeAsync(connection, "ALTER SESSION SET TIME_OUTPUT_FORMAT = 'HH12:MI:SS.FF9'");
      });

      afterAll(async () => {
        await executeAsync(connection, 'ALTER SESSION UNSET TIME_OUTPUT_FORMAT');
      });

      it('wraps HH12 to a 12-hour clock, not alias it to HH24', async () => {
        const { rows } = await executeAsync(
          connection,
          `SELECT '08:15:30.789789789'::TIME, '14:45:30.789789789'::TIME`,
        );
        expect(Object.values(rows![0])).toEqual(['08:15:30.789789789', '02:45:30.789789789']);
      });
    });

    describe('HH24:MI:SS.FF6 format', () => {
      beforeAll(async () => {
        await executeAsync(connection, "ALTER SESSION SET TIME_OUTPUT_FORMAT = 'HH24:MI:SS.FF6'");
      });

      afterAll(async () => {
        await executeAsync(connection, 'ALTER SESSION UNSET TIME_OUTPUT_FORMAT');
      });

      it('selects microseconds when TIME_OUTPUT_FORMAT includes FF', async () => {
        const { rows } = await executeAsync(
          connection,
          `SELECT '10:30:00'::TIME, '10:30:00.123456'::TIME, '23:59:59.999999'::TIME`,
        );
        expect(Object.values(rows![0])).toEqual([
          '10:30:00.000000',
          '10:30:00.123456',
          '23:59:59.999999',
        ]);
      });
    });

    describe('HH24:MI:SS.FF format, scale-0 column', () => {
      beforeAll(async () => {
        await executeAsync(connection, "ALTER SESSION SET TIME_OUTPUT_FORMAT = 'HH24:MI:SS.FF'");
      });

      afterAll(async () => {
        await executeAsync(connection, 'ALTER SESSION UNSET TIME_OUTPUT_FORMAT');
      });

      it('drops a dangling "." for FF against a scale-0 column, unlike the legacy driver', async () => {
        const { rows } = await executeAsync(connection, "SELECT '10:30:00'::TIME(0)");
        if (isRunningNewDriverWithBD('BD#15')) {
          expect(Object.values(rows![0])).toEqual(['10:30:00']);
        } else {
          expect(Object.values(rows![0])).toEqual(['10:30:00.']);
        }
      });
    });

    describe('HH:MI:SS.FF3 format', () => {
      beforeAll(async () => {
        await executeAsync(connection, "ALTER SESSION SET TIME_OUTPUT_FORMAT = 'HH:MI:SS.FF3'");
      });

      afterAll(async () => {
        await executeAsync(connection, 'ALTER SESSION UNSET TIME_OUTPUT_FORMAT');
      });

      it('renders bare HH the same as HH24', async () => {
        const { rows } = await executeAsync(connection, "SELECT '14:45:30.123'::TIME");
        expect(Object.values(rows![0])).toEqual(['14:45:30.123']);
      });
    });

    describe('HH12:MI:SS AM format', () => {
      beforeAll(async () => {
        await executeAsync(connection, "ALTER SESSION SET TIME_OUTPUT_FORMAT = 'HH12:MI:SS AM'");
      });

      afterAll(async () => {
        await executeAsync(connection, 'ALTER SESSION UNSET TIME_OUTPUT_FORMAT');
      });

      it('renders AM/PM from the clock, not the token letters', async () => {
        const { rows } = await executeAsync(
          connection,
          `SELECT '08:15:30'::TIME, '14:45:30'::TIME`,
        );
        expect(Object.values(rows![0])).toEqual(['08:15:30 AM', '02:45:30 PM']);
      });
    });

    describe('YYYY-MM-DD HH24:MI:SS format', () => {
      beforeAll(async () => {
        await executeAsync(
          connection,
          "ALTER SESSION SET TIME_OUTPUT_FORMAT = 'YYYY-MM-DD HH24:MI:SS'",
        );
      });

      afterAll(async () => {
        await executeAsync(connection, 'ALTER SESSION UNSET TIME_OUTPUT_FORMAT');
      });

      it('renders epoch calendar tokens, not the token letters', async () => {
        const { rows } = await executeAsync(connection, "SELECT '14:45:30'::TIME");
        expect(Object.values(rows![0])).toEqual(['1970-01-01 14:45:30']);
      });
    });

    describe('HH24:MI:SS TZHTZM format', () => {
      beforeAll(async () => {
        await executeAsync(
          connection,
          "ALTER SESSION SET TIME_OUTPUT_FORMAT = 'HH24:MI:SS TZHTZM'",
        );
      });

      afterAll(async () => {
        await executeAsync(connection, 'ALTER SESSION UNSET TIME_OUTPUT_FORMAT');
      });

      it('renders UTC offset tokens, not the token letters', async () => {
        const { rows } = await executeAsync(connection, "SELECT '14:45:30'::TIME");
        expect(Object.values(rows![0])).toEqual(['14:45:30 +0000']);
      });
    });

    describe('MMMM DD, YYYY format', () => {
      beforeAll(async () => {
        await executeAsync(connection, "ALTER SESSION SET TIME_OUTPUT_FORMAT = 'MMMM DD, YYYY'");
      });

      afterAll(async () => {
        await executeAsync(connection, 'ALTER SESSION UNSET TIME_OUTPUT_FORMAT');
      });

      // BCR_LOG: TIME has no month; this copies the old converter accident.
      it('renders MMMM as January at Unix epoch', async () => {
        const { rows } = await executeAsync(connection, "SELECT '14:45:30'::TIME");
        expect(Object.values(rows![0])).toEqual(['January 01, 1970']);
      });
    });
  });

  describe.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)('TIMESTAMP', () => {
    it('returns TIMESTAMP_LTZ as Date with zone offset', async () => {
      const { statement, rows } = await executeAsync(
        connection,
        "SELECT to_timestamp_ltz('Thu, 21 Jan 2016 06:32:44 -0800') as LTZ_COLUMN",
      );
      const column = getStatementColumn(statement, 0);
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
      const column = getStatementColumn(statement, 0);
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
      const column = getStatementColumn(statement, 0);
      const selectedValue = rows![0].NTZ_COLUMN as Date;
      expect(column.isTimestamp()).toBe(true);
      expect(column.getType()).toBe('timestamp_ntz');
      expect(selectedValue).toBeInstanceOf(Date);
      expect(selectedValue.toJSON()).toBe('2016-01-21 06:32:44.000');
    });
  });
});
