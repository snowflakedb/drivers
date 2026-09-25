import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import type { Connection, SnowflakeDate } from '../../../types/sdk-types.js';
import { createLiveConnection, createTemporaryTable } from '../../utils/fixtures.js';
import {
  dateAtUtc,
  destroyConnectionAsync,
  executeAsync,
  getStatementColumn,
  NOT_IMPLEMENTED_IN_NEW_DRIVER,
} from '../../utils/index.js';
import { setSessionParameterForTest } from '../../utils/query.js';
import { createLiveNullPreservingConnection } from '../utils.js';

type ExpectedTzValue = null | { date: Date; offsetMinutes: number; nanoSeconds?: number };

function expectTzDates(values: unknown[], expected: ExpectedTzValue[]): void {
  expect(values).toHaveLength(expected.length);
  expected.forEach((entry, index) => {
    const value = values[index];
    if (entry === null) {
      expect(value).toBeNull();
      return;
    }
    const actual = value as SnowflakeDate;
    expect(actual).toBeInstanceOf(Date);
    expect(actual).toEqual(entry.date);
    expect(actual.getTimezone()).toBe(entry.offsetMinutes);
    if (entry.nanoSeconds !== undefined) {
      expect(actual.getNanoSeconds()).toBe(entry.nanoSeconds);
    }
  });
}

describe.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)('TIMESTAMP_TZ data type', () => {
  let connection: Connection;

  beforeAll(async () => {
    connection = await createLiveConnection({}, false);
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  describe('tests/definitions/shared/types/timestamp_tz.feature', () => {
    it('should cast timestamp_tz values to appropriate type', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT '2024-01-15 10:30:00 +05:00'::TIMESTAMP_TZ" is executed
      const { statement, rows } = await executeAsync(
        connection,
        `SELECT '2024-01-15 10:30:00 +05:00'::TIMESTAMP_TZ`,
      );
      const column = getStatementColumn(statement, 0);
      const value = Object.values(rows[0])[0];

      // Then All values should be returned as appropriate type
      expect(column.getType()).toBe('timestamp_tz');
      expect(column.isTimestamp()).toBe(true);
      expect(column.isTimestampTz()).toBe(true);

      // And Values should have timezone info
      expectTzDates([value], [{ date: dateAtUtc('2024-01-15T05:30:00'), offsetMinutes: 300 }]);
    });

    it.each<{ values: string; query: string; expected: ExpectedTzValue[] }>([
      {
        values: 'basic',
        query: `'2024-01-15 10:30:00 +05:00'::TIMESTAMP_TZ, '2024-06-20 14:45:30 -08:00'::TIMESTAMP_TZ`,
        expected: [
          { date: dateAtUtc('2024-01-15T05:30:00'), offsetMinutes: 300 },
          { date: dateAtUtc('2024-06-20T22:45:30'), offsetMinutes: -480 },
        ],
      },
      {
        values: 'epoch',
        query: `'1970-01-01 00:00:00 +00:00'::TIMESTAMP_TZ`,
        expected: [{ date: dateAtUtc('1970-01-01T00:00:00'), offsetMinutes: 0 }],
      },
      {
        values: 'microseconds',
        query: `'2024-01-15 10:30:00.123456 +05:00'::TIMESTAMP_TZ`,
        expected: [
          {
            date: dateAtUtc('2024-01-15T05:30:00.123'),
            offsetMinutes: 300,
            nanoSeconds: 123456000,
          },
        ],
      },
    ])('should select timestamp_tz $values', async ({ query, expected }) => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT <query_values>" is executed
      const { rows } = await executeAsync(connection, `SELECT ${query}`);

      // Then Result should contain timestamps <expected_values>
      // And Values should have timezone info
      expectTzDates(Object.values(rows[0]), expected);
    });

    it('should select sub-second timestamp_tz values before epoch', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Sub-second timestamp_tz values before the epoch are selected
      const { rows } = await executeAsync(
        connection,
        `SELECT '1969-12-31 23:59:59.999999999 +00:00'::TIMESTAMP_TZ AS NEAR_EPOCH,
                '1969-12-31 23:59:58.5 +00:00'::TIMESTAMP_TZ(3) AS SCALE_3`,
      );

      // Then Result should contain the expected sub-second values before the epoch
      expectTzDates(
        [rows[0].NEAR_EPOCH, rows[0].SCALE_3],
        [
          { date: dateAtUtc('1969-12-31T23:59:59.999'), offsetMinutes: 0, nanoSeconds: 999999999 },
          { date: dateAtUtc('1969-12-31T23:59:58.500'), offsetMinutes: 0, nanoSeconds: 500000000 },
        ],
      );
    });

    it('should preserve timezone offset for timestamp_tz', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT '2024-01-15 10:30:00 +05:30'::TIMESTAMP_TZ, '2024-01-15 10:30:00 -08:00'::TIMESTAMP_TZ, '2024-01-15 10:30:00 +00:00'::TIMESTAMP_TZ, '2024-01-15 10:30:00 +04:30'::TIMESTAMP_TZ, '2024-01-15 10:30:00 -02:30'::TIMESTAMP_TZ" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT '2024-01-15 10:30:00 +05:30'::TIMESTAMP_TZ,
                '2024-01-15 10:30:00 -08:00'::TIMESTAMP_TZ,
                '2024-01-15 10:30:00 +00:00'::TIMESTAMP_TZ,
                '2024-01-15 10:30:00 +04:30'::TIMESTAMP_TZ,
                '2024-01-15 10:30:00 -02:30'::TIMESTAMP_TZ`,
      );

      // Then Result should preserve offsets [+05:30, -08:00, +00:00, +04:30, -02:30]
      expectTzDates(Object.values(rows[0]), [
        { date: dateAtUtc('2024-01-15T05:00:00'), offsetMinutes: 330 },
        { date: dateAtUtc('2024-01-15T18:30:00'), offsetMinutes: -480 },
        { date: dateAtUtc('2024-01-15T10:30:00'), offsetMinutes: 0 },
        { date: dateAtUtc('2024-01-15T06:00:00'), offsetMinutes: 270 },
        { date: dateAtUtc('2024-01-15T13:00:00'), offsetMinutes: -150 },
      ]);
    });

    it.each<{ values: string; query: string; expected: ExpectedTzValue[] }>([
      {
        values: 'year 9999',
        query: `'9999-12-31 23:59:59 +00:00'::TIMESTAMP_TZ`,
        expected: [{ date: dateAtUtc('9999-12-31T23:59:59'), offsetMinutes: 0 }],
      },
      {
        values: 'year 1900',
        query: `'1900-01-01 00:00:00 +00:00'::TIMESTAMP_TZ`,
        expected: [{ date: dateAtUtc('1900-01-01T00:00:00'), offsetMinutes: 0 }],
      },
      {
        values: 'pre-epoch',
        query: `'1960-06-15 12:00:00 +05:00'::TIMESTAMP_TZ`,
        expected: [{ date: dateAtUtc('1960-06-15T07:00:00'), offsetMinutes: 300 }],
      },
    ])('should select edge date timestamp_tz $values', async ({ query, expected }) => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT <query_values>" is executed
      const { rows } = await executeAsync(connection, `SELECT ${query}`);

      // Then Result should contain timestamps <expected_values>
      // And Values should have timezone info
      expectTzDates(Object.values(rows[0]), expected);
    });

    it('should handle NULL values for timestamp_tz', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT '2024-01-15 10:30:00 +05:00'::TIMESTAMP_TZ, NULL::TIMESTAMP_TZ" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT '2024-01-15 10:30:00 +05:00'::TIMESTAMP_TZ, NULL::TIMESTAMP_TZ`,
      );

      // Then Result should contain [2024-01-15 10:30:00 +05:00, NULL]
      expectTzDates(Object.values(rows[0]), [
        { date: dateAtUtc('2024-01-15T05:30:00'), offsetMinutes: 300 },
        null,
      ]);
    });

    it.each<{ values: string; insert: string; expected: ExpectedTzValue[] }>([
      {
        values: 'basic',
        insert: `('2024-01-15 10:30:00 +05:00'), ('2024-06-20 14:45:30 -08:00')`,
        expected: [
          { date: dateAtUtc('2024-01-15T05:30:00'), offsetMinutes: 300 },
          { date: dateAtUtc('2024-06-20T22:45:30'), offsetMinutes: -480 },
        ],
      },
      {
        values: 'epoch',
        insert: `('1970-01-01 00:00:00 +00:00'), ('2024-01-15 10:30:00 +05:00')`,
        expected: [
          { date: dateAtUtc('1970-01-01T00:00:00'), offsetMinutes: 0 },
          { date: dateAtUtc('2024-01-15T05:30:00'), offsetMinutes: 300 },
        ],
      },
      {
        values: 'microseconds',
        insert: `('2024-01-15 10:30:00 +05:00'), ('2024-01-15 10:30:00.123456 +05:00')`,
        expected: [
          { date: dateAtUtc('2024-01-15T05:30:00'), offsetMinutes: 300 },
          {
            date: dateAtUtc('2024-01-15T05:30:00.123'),
            offsetMinutes: 300,
            nanoSeconds: 123456000,
          },
        ],
      },
      {
        values: 'null',
        insert: `(NULL), ('2024-01-15 10:30:00 +05:00')`,
        expected: [{ date: dateAtUtc('2024-01-15T05:30:00'), offsetMinutes: 300 }, null],
      },
    ])('should select $values from table for timestamp_tz', async ({ insert, expected }) => {
      // Given Snowflake client is logged in
      void connection;

      // And Table with TIMESTAMP_TZ column exists with values <insert_values>
      const tableName = await createTemporaryTable(connection, 'COL TIMESTAMP_TZ');
      await executeAsync(connection, `INSERT INTO ${tableName} (COL) VALUES ${insert}`);

      // When Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT * FROM ${tableName} ORDER BY COL NULLS LAST`,
      );

      // Then Result should contain timestamps <expected_values>
      // And Values should have timezone info
      expectTzDates(
        rows.map((row) => row.COL),
        expected,
      );
    });

    describe('parameter binding', () => {
      it('should select timestamp_tz using parameter binding', async () => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT ?::TIMESTAMP_TZ, ?::TIMESTAMP_TZ" is executed with bound timestamp values
        const { rows } = await executeAsync(
          connection,
          `SELECT ?::TIMESTAMP_TZ AS COL1, ?::TIMESTAMP_TZ AS COL2`,
          { binds: ['2024-01-15 10:30:00 +05:00', '2024-06-20 14:45:30 -08:00'] },
        );

        // Then Result should contain the bound timestamps
        void 0;
        // And Values should have timezone info
        expectTzDates(Object.values(rows[0]), [
          { date: dateAtUtc('2024-01-15T05:30:00'), offsetMinutes: 300 },
          { date: dateAtUtc('2024-06-20T22:45:30'), offsetMinutes: -480 },
        ]);
      });

      it('should select null timestamp_tz using parameter binding', async () => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT ?::TIMESTAMP_TZ" is executed with bound NULL value
        const { rows } = await executeAsync(connection, `SELECT ?::TIMESTAMP_TZ`, {
          binds: [null],
        });

        // Then Result should contain [NULL]
        expectTzDates(Object.values(rows[0]), [null]);
      });

      it('should insert timestamp_tz using parameter binding', async () => {
        // Given Snowflake client is logged in
        void connection;

        // And Table with TIMESTAMP_TZ column exists
        const tableName = await createTemporaryTable(connection, 'COL TIMESTAMP_TZ');

        // When Timestamp values are bulk-inserted using multirow binding
        await executeAsync(connection, `INSERT INTO ${tableName} (COL) VALUES (?)`, {
          binds: [['2024-01-15 10:30:00 +05:00'], ['2024-06-20 14:45:30 -08:00'], [null]],
        });

        // And Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
        const { rows } = await executeAsync(
          connection,
          `SELECT * FROM ${tableName} ORDER BY COL NULLS LAST`,
        );

        // Then SELECT should return the same values in any order
        expectTzDates(
          rows.map((row) => row.COL),
          [
            { date: dateAtUtc('2024-01-15T05:30:00'), offsetMinutes: 300 },
            { date: dateAtUtc('2024-06-20T22:45:30'), offsetMinutes: -480 },
            null,
          ],
        );
      });
    });

    describe('TIMEZONE Parameter', () => {
      it('should apply the session TIMEZONE to a timestamp_tz literal without an explicit offset', async () => {
        // Given Snowflake client is logged in
        void connection;

        // And Session TIMEZONE is set to Europe/Warsaw
        await setSessionParameterForTest(connection, 'TIMEZONE', 'Europe/Warsaw');

        // When Query "SELECT '2024-01-15 10:30:00'::TIMESTAMP_TZ" is executed
        const { rows } = await executeAsync(
          connection,
          `SELECT '2024-01-15 10:30:00'::TIMESTAMP_TZ`,
        );

        // Then Result should contain timestamps 2024-01-15 10:30:00 +01:00
        void 0;
        // And Values should have timezone info
        expectTzDates(Object.values(rows[0]), [
          { date: dateAtUtc('2024-01-15T09:30:00'), offsetMinutes: 60 },
        ]);
      });

      it('should keep the explicit offset of a timestamp_tz literal regardless of the session TIMEZONE', async () => {
        // Given Snowflake client is logged in
        void connection;

        // And Session TIMEZONE is set to Europe/Warsaw
        await setSessionParameterForTest(connection, 'TIMEZONE', 'Europe/Warsaw');

        // When Query "SELECT '2024-01-15 10:30:00 +05:00'::TIMESTAMP_TZ" is executed
        const { rows } = await executeAsync(
          connection,
          `SELECT '2024-01-15 10:30:00 +05:00'::TIMESTAMP_TZ`,
        );

        // Then Result should contain timestamps 2024-01-15 10:30:00 +05:00
        void 0;

        // And Values should have timezone info
        expectTzDates(Object.values(rows[0]), [
          { date: dateAtUtc('2024-01-15T05:30:00'), offsetMinutes: 300 },
        ]);
      });

      it('should let a statement-level TIMEZONE override the session for a timestamp_tz literal without an explicit offset', async () => {
        // Given Snowflake client is logged in
        void connection;

        // And Session TIMEZONE is set to Europe/Warsaw
        await setSessionParameterForTest(connection, 'TIMEZONE', 'Europe/Warsaw');

        // When Query "SELECT '2024-01-15 10:30:00'::TIMESTAMP_TZ" is executed with statement TIMEZONE America/New_York
        const { rows } = await executeAsync(
          connection,
          `SELECT '2024-01-15 10:30:00'::TIMESTAMP_TZ`,
          { parameters: { TIMEZONE: 'America/New_York' } },
        );

        // Then Result should contain timestamps 2024-01-15 10:30:00 -05:00
        void 0;

        // And Values should have timezone info
        expectTzDates(Object.values(rows[0]), [
          { date: dateAtUtc('2024-01-15T15:30:00'), offsetMinutes: -300 },
        ]);
      });
    });

    describe.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)('multiple chunks', () => {
      const rowCount = 50_000;

      it('should download large result set with multiple chunks for timestamp_tz', async () => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1, '2024-01-01 00:00:00 +00:00'::TIMESTAMP_TZ) as ts FROM TABLE(GENERATOR(ROWCOUNT => 50000)) ORDER BY ts" is executed
        const { rows } = await executeAsync(
          connection,
          `SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1, '2024-01-01 00:00:00 +00:00'::TIMESTAMP_TZ) AS TS FROM TABLE(GENERATOR(ROWCOUNT => ${rowCount})) ORDER BY TS`,
        );

        // Then Result should contain 50000 sequentially increasing timestamps from 2024-01-01 00:00:00 +00:00
        expect(rows).toHaveLength(rowCount);
        expectTzDates(
          [rows[0].TS, rows[rowCount - 1].TS],
          [
            { date: dateAtUtc('2024-01-01T00:00:00'), offsetMinutes: 0 },
            {
              date: new Date(dateAtUtc('2024-01-01T00:00:00').getTime() + (rowCount - 1) * 1000),
              offsetMinutes: 0,
            },
          ],
        );
      });

      it('should download large result set with multiple chunks from table for timestamp_tz', async () => {
        // Given Snowflake client is logged in
        void connection;

        // And Table with TIMESTAMP_TZ column exists with 50000 sequential timestamp values
        const tableName = await createTemporaryTable(connection, 'COL TIMESTAMP_TZ');
        await executeAsync(
          connection,
          `INSERT INTO ${tableName} (COL)
             SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1, '2024-01-01 00:00:00 +00:00'::TIMESTAMP_TZ)
             FROM TABLE(GENERATOR(ROWCOUNT => ${rowCount}))`,
        );

        // When Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
        const { rows } = await executeAsync(
          connection,
          `SELECT * FROM ${tableName} ORDER BY COL NULLS LAST`,
        );

        // Then Result should contain 50000 sequentially increasing timestamps from 2024-01-01 00:00:00 +00:00
        expect(rows).toHaveLength(rowCount);
        expectTzDates(
          [rows[0].COL, rows[rowCount - 1].COL],
          [
            { date: dateAtUtc('2024-01-01T00:00:00'), offsetMinutes: 0 },
            {
              date: new Date(dateAtUtc('2024-01-01T00:00:00').getTime() + (rowCount - 1) * 1000),
              offsetMinutes: 0,
            },
          ],
        );
      });
    });
  });

  describe('fetchAsString', () => {
    it('should render timestamp_tz as YYYY-MM-DD HH24:MI:SS.FF3 TZHTZM', async () => {
      const { rows } = await executeAsync(
        connection,
        `SELECT '2024-01-15 10:30:00 +05:00'::TIMESTAMP_TZ`,
        { fetchAsString: ['Date'] },
      );
      expect(Object.values(rows[0])).toEqual(['2024-01-15 10:30:00.000 +0500']);
    });

    it("should render a NULL TIMESTAMP_TZ cell as the string 'NULL'", async () => {
      const { rows } = await executeAsync(connection, `SELECT NULL::TIMESTAMP_TZ`, {
        fetchAsString: ['Date'],
      });
      expect(Object.values(rows[0])).toEqual(['NULL']);
    });

    it('should render a NULL TIMESTAMP_TZ cell as null when representNullAsStringNull is disabled', async () => {
      const nullPreservingConnection = await createLiveNullPreservingConnection();
      const { rows } = await executeAsync(nullPreservingConnection, `SELECT NULL::TIMESTAMP_TZ`, {
        fetchAsString: ['Date'],
      });
      expect(Object.values(rows[0])).toEqual([null]);
    });
  });

  describe('TIMESTAMP_TZ_OUTPUT_FORMAT and TIMESTAMP_OUTPUT_FORMAT', () => {
    async function selectTz(
      literal: string,
      options?: Parameters<typeof executeAsync>[2],
    ): Promise<unknown> {
      const { rows } = await executeAsync(connection, `SELECT '${literal}'::TIMESTAMP_TZ`, options);
      return Object.values(rows[0])[0];
    }

    // statement-level is a known bug in both drivers documented in BCR_LOG.md
    it.each([
      {
        name: 'TZ only',
        parameters: { TIMESTAMP_TZ_OUTPUT_FORMAT: 'YYYY-MM-DD HH24:MI:SS.FF6 TZH:TZM' },
      },
      { name: 'generic only', parameters: { TIMESTAMP_OUTPUT_FORMAT: 'YYYY-MM-DD HH24:MI' } },
      {
        name: 'both',
        parameters: {
          TIMESTAMP_TZ_OUTPUT_FORMAT: 'YYYY-MM-DD HH24:MI:SS.FF6 TZH:TZM',
          TIMESTAMP_OUTPUT_FORMAT: 'YYYY-MM-DD HH24:MI',
        },
      },
    ])('should ignore statement-level $name output format', async ({ parameters }) => {
      expect(
        await selectTz('2024-01-15 10:30:00 +05:00', { parameters, fetchAsString: ['Date'] }),
      ).toBe('2024-01-15 10:30:00.000 +0500');

      const date = (await selectTz('2024-01-15 10:30:00 +05:00', { parameters })) as SnowflakeDate;
      expect(date.toJSON()).toBe('2024-01-15 10:30:00.000 +0500');
      expect(date.getFormat()).toBe('YYYY-MM-DD HH24:MI:SS.FF3 TZHTZM');
    });

    describe('session-level', () => {
      it('should honor TIMESTAMP_OUTPUT_FORMAT when TIMESTAMP_TZ_OUTPUT_FORMAT is empty', async () => {
        await setSessionParameterForTest(connection, 'TIMESTAMP_TZ_OUTPUT_FORMAT', '');
        await setSessionParameterForTest(
          connection,
          'TIMESTAMP_OUTPUT_FORMAT',
          'YYYY-MM-DD HH24:MI TZH:TZM',
        );

        expect(await selectTz('2024-01-15 10:30:00 +05:00', { fetchAsString: ['Date'] })).toBe(
          '2024-01-15 10:30 +05:00',
        );

        const date = (await selectTz('2024-01-15 10:30:00 +05:00')) as SnowflakeDate;
        expect(date.toJSON()).toBe('2024-01-15 10:30 +05:00');
        expect(date.getFormat()).toBe('YYYY-MM-DD HH24:MI TZH:TZM');
      });

      it('should honor TIMESTAMP_TZ_OUTPUT_FORMAT', async () => {
        await setSessionParameterForTest(
          connection,
          'TIMESTAMP_TZ_OUTPUT_FORMAT',
          'YYYY-MM-DD HH24:MI:SS.FF6 TZH:TZM',
        );

        expect(await selectTz('2024-01-15 10:30:00 +05:00', { fetchAsString: ['Date'] })).toBe(
          '2024-01-15 10:30:00.000000 +05:00',
        );

        const date = (await selectTz('2024-01-15 10:30:00 +05:00')) as SnowflakeDate;
        expect(date.toJSON()).toBe('2024-01-15 10:30:00.000000 +05:00');
        expect(date.getFormat()).toBe('YYYY-MM-DD HH24:MI:SS.FF6 TZH:TZM');
      });

      it('should prefer TIMESTAMP_TZ_OUTPUT_FORMAT when both timestamp output formats are set', async () => {
        await setSessionParameterForTest(
          connection,
          'TIMESTAMP_OUTPUT_FORMAT',
          'YYYY-MM-DD HH24:MI',
        );
        await setSessionParameterForTest(
          connection,
          'TIMESTAMP_TZ_OUTPUT_FORMAT',
          'YYYY-MM-DD HH24:MI:SS.FF6 TZH:TZM',
        );

        expect(await selectTz('2024-01-15 10:30:00 +05:00', { fetchAsString: ['Date'] })).toBe(
          '2024-01-15 10:30:00.000000 +05:00',
        );

        const date = (await selectTz('2024-01-15 10:30:00 +05:00')) as SnowflakeDate;
        expect(date.toJSON()).toBe('2024-01-15 10:30:00.000000 +05:00');
        expect(date.getFormat()).toBe('YYYY-MM-DD HH24:MI:SS.FF6 TZH:TZM');
      });
    });
  });

  it('should expose SnowflakeDate custom methods', async () => {
    const { rows } = await executeAsync(
      connection,
      `SELECT '2024-01-15 10:30:00.123456789 +05:00'::TIMESTAMP_TZ AS VAL`,
    );
    const date = rows[0].VAL as SnowflakeDate;
    expect(date).toBeInstanceOf(Date);
    expect(date.getEpochSeconds()).toBe(Date.UTC(2024, 0, 15, 5, 30, 0) / 1000);
    expect(date.getNanoSeconds()).toBe(123456789);
    expect(date.getScale()).toBe(9);
    expect(date.getTimezone()).toBe(300);
    expect(date.getFormat()).toBe('YYYY-MM-DD HH24:MI:SS.FF3 TZHTZM');
    expect(date.toJSON()).toBe('2024-01-15 10:30:00.123 +0500');
  });
});
