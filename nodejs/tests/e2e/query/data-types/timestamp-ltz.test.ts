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
import { setSessionParameter, setSessionParameterForTest } from '../../utils/query.js';
import { createLiveNullPreservingConnection } from '../utils.js';

const SESSION_TIMEZONE = 'America/New_York';

type ExpectedLtzValue = Date | null | { date: Date | null; nanoSeconds?: number };

function expectLtzDates(values: unknown[], expected: ExpectedLtzValue[]): void {
  expect(values).toHaveLength(expected.length);
  expected.forEach((entry, index) => {
    const { date, nanoSeconds } =
      entry === null || entry instanceof Date ? { date: entry, nanoSeconds: undefined } : entry;
    const value = values[index];
    if (date === null) {
      expect(value).toBeNull();
      return;
    }
    const actual = value as SnowflakeDate;
    expect(actual).toBeInstanceOf(Date);
    expect(actual).toEqual(date);
    expect(actual.getTimezone()).toBe(SESSION_TIMEZONE);
    if (nanoSeconds !== undefined) {
      expect(actual.getNanoSeconds()).toBe(nanoSeconds);
    }
  });
}

describe.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)('TIMESTAMP_LTZ data type', () => {
  let connection: Connection;

  beforeAll(async () => {
    connection = await createLiveConnection({}, false);
    await setSessionParameter(connection, 'TIMEZONE', SESSION_TIMEZONE);
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  describe('tests/definitions/shared/types/timestamp_ltz.feature', () => {
    it('should cast timestamp_ltz values to appropriate type', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT '2024-01-15 10:30:00 +00:00'::TIMESTAMP_LTZ" is executed
      const { statement, rows } = await executeAsync(
        connection,
        `SELECT '2024-01-15 10:30:00 +00:00'::TIMESTAMP_LTZ`,
      );
      const column = getStatementColumn(statement, 0);
      const value = Object.values(rows[0])[0];

      // Then All values should be returned as appropriate type
      expect(column.getType()).toBe('timestamp_ltz');
      expect(column.isTimestamp()).toBe(true);
      expect(column.isTimestampLtz()).toBe(true);

      // And Values should have timezone info
      expectLtzDates([value], [dateAtUtc('2024-01-15T10:30:00')]);
    });

    it.each<{ values: string; query: string; expected: ExpectedLtzValue[] }>([
      {
        values: 'basic',
        query: `'2024-01-15 10:30:00 +00:00'::TIMESTAMP_LTZ, '2024-06-20 14:45:30 +00:00'::TIMESTAMP_LTZ`,
        expected: [dateAtUtc('2024-01-15T10:30:00'), dateAtUtc('2024-06-20T14:45:30')],
      },
      {
        values: 'epoch',
        query: `'1970-01-01 00:00:00 +00:00'::TIMESTAMP_LTZ`,
        expected: [dateAtUtc('1970-01-01T00:00:00')],
      },
      {
        values: 'microseconds',
        query: `'2024-01-15 10:30:00.123456 +00:00'::TIMESTAMP_LTZ`,
        expected: [{ date: dateAtUtc('2024-01-15T10:30:00.123'), nanoSeconds: 123456000 }],
      },
    ])('should select timestamp_ltz $values', async ({ query, expected }) => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT <query_values>" is executed
      const { rows } = await executeAsync(connection, `SELECT ${query}`);

      // Then Result should contain timestamps <expected_values>
      expectLtzDates(Object.values(rows[0]), expected);
    });

    it('should select sub-second timestamp_ltz values before epoch', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Sub-second timestamp_ltz values before the epoch are selected
      const { rows } = await executeAsync(
        connection,
        `SELECT '1969-12-31 23:59:59.999999999 +00:00'::TIMESTAMP_LTZ AS NEAR_EPOCH,
                '1969-12-31 23:59:58.5 +00:00'::TIMESTAMP_LTZ(3) AS SCALE_3`,
      );

      // Then Result should contain the expected sub-second values before the epoch
      expectLtzDates(
        [rows[0].NEAR_EPOCH, rows[0].SCALE_3],
        [
          { date: dateAtUtc('1969-12-31T23:59:59.999'), nanoSeconds: 999999999 },
          { date: dateAtUtc('1969-12-31T23:59:58.500'), nanoSeconds: 500000000 },
        ],
      );
    });

    it('should handle NULL values for timestamp_ltz', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT '2024-01-15 10:30:00 +00:00'::TIMESTAMP_LTZ, NULL::TIMESTAMP_LTZ" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT '2024-01-15 10:30:00 +00:00'::TIMESTAMP_LTZ, NULL::TIMESTAMP_LTZ`,
      );

      // Then Result should contain [2024-01-15 10:30:00 UTC, NULL]
      expectLtzDates(Object.values(rows[0]), [dateAtUtc('2024-01-15T10:30:00'), null]);
    });

    it.each<{ values: string; insert: string; expected: ExpectedLtzValue[] }>([
      {
        values: 'basic',
        insert: `('2024-01-15 10:30:00 +00:00'), ('2024-06-20 14:45:30 +00:00')`,
        expected: [dateAtUtc('2024-01-15T10:30:00'), dateAtUtc('2024-06-20T14:45:30')],
      },
      {
        values: 'epoch',
        insert: `('1970-01-01 00:00:00 +00:00'), ('2024-01-15 10:30:00 +00:00')`,
        expected: [dateAtUtc('1970-01-01T00:00:00'), dateAtUtc('2024-01-15T10:30:00')],
      },
      {
        values: 'null',
        insert: `(NULL), ('2024-01-15 10:30:00 +00:00')`,
        expected: [dateAtUtc('2024-01-15T10:30:00'), null],
      },
    ])('should select $values from table for timestamp_ltz', async ({ insert, expected }) => {
      // Given Snowflake client is logged in
      void connection;

      // And Table with TIMESTAMP_LTZ column exists with values <insert_values>
      const tableName = await createTemporaryTable(connection, 'COL TIMESTAMP_LTZ');
      await executeAsync(connection, `INSERT INTO ${tableName} (COL) VALUES ${insert}`);

      // When Query "SELECT * FROM <table> ORDER BY col" is executed
      const { rows } = await executeAsync(connection, `SELECT * FROM ${tableName} ORDER BY COL`);

      // Then Result should contain timestamps <expected_values>
      expectLtzDates(
        rows.map((row) => row.COL),
        expected,
      );
    });

    describe('parameter binding', () => {
      it('should select timestamp_ltz using parameter binding', async () => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT ?::TIMESTAMP_LTZ, ?::TIMESTAMP_LTZ" is executed with bound timestamp values
        const { rows } = await executeAsync(
          connection,
          `SELECT ?::TIMESTAMP_LTZ AS COL1, ?::TIMESTAMP_LTZ AS COL2`,
          { binds: ['2024-01-15 10:30:00 +00:00', '2024-06-20 14:45:30 +00:00'] },
        );

        // Then Result should contain the bound timestamps
        expectLtzDates(Object.values(rows[0]), [
          dateAtUtc('2024-01-15T10:30:00'),
          dateAtUtc('2024-06-20T14:45:30'),
        ]);
      });

      it('should select null timestamp_ltz using parameter binding', async () => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT ?::TIMESTAMP_LTZ" is executed with bound NULL value
        const { rows } = await executeAsync(connection, `SELECT ?::TIMESTAMP_LTZ`, {
          binds: [null],
        });

        // Then Result should contain [NULL]
        expectLtzDates(Object.values(rows[0]), [null]);
      });

      it('should insert timestamp_ltz using parameter binding', async () => {
        // Given Snowflake client is logged in
        void connection;

        // And Table with TIMESTAMP_LTZ column exists
        const tableName = await createTemporaryTable(connection, 'COL TIMESTAMP_LTZ');

        // When Timestamp values are bulk-inserted using multirow binding
        await executeAsync(connection, `INSERT INTO ${tableName} (COL) VALUES (?)`, {
          binds: [['2024-01-15 10:30:00 +00:00'], ['2024-06-20 14:45:30 +00:00'], [null]],
        });

        // And Query "SELECT * FROM <table> ORDER BY col" is executed
        const { rows } = await executeAsync(connection, `SELECT * FROM ${tableName} ORDER BY COL`);

        // Then SELECT should return the same values in any order
        expectLtzDates(
          rows.map((row) => row.COL),
          [dateAtUtc('2024-01-15T10:30:00'), dateAtUtc('2024-06-20T14:45:30'), null],
        );
      });
    });

    describe.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)('multiple chunks', () => {
      const rowCount = 50_000;

      it('should download large result set with multiple chunks for timestamp_ltz', async () => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1, '2024-01-01 00:00:00 +00:00'::TIMESTAMP_LTZ) as ts FROM TABLE(GENERATOR(ROWCOUNT => 50000)) ORDER BY ts" is executed
        const { rows } = await executeAsync(
          connection,
          `SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1, '2024-01-01 00:00:00 +00:00'::TIMESTAMP_LTZ) AS TS FROM TABLE(GENERATOR(ROWCOUNT => ${rowCount})) ORDER BY TS`,
        );

        // Then Result should contain 50000 sequentially increasing timestamps from 2024-01-01 00:00:00 UTC
        expect(rows).toHaveLength(rowCount);
        expectLtzDates(
          [rows[0].TS, rows[rowCount - 1].TS],
          [
            dateAtUtc('2024-01-01T00:00:00'),
            new Date(dateAtUtc('2024-01-01T00:00:00').getTime() + (rowCount - 1) * 1000),
          ],
        );
      });

      it('should download large result set with multiple chunks from table for timestamp_ltz', async () => {
        // Given Snowflake client is logged in
        void connection;

        // And Table with TIMESTAMP_LTZ column exists with 50000 sequential timestamp values
        const tableName = await createTemporaryTable(connection, 'COL TIMESTAMP_LTZ');
        await executeAsync(
          connection,
          `INSERT INTO ${tableName} (COL)
             SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1, '2024-01-01 00:00:00 +00:00'::TIMESTAMP_LTZ)
             FROM TABLE(GENERATOR(ROWCOUNT => ${rowCount}))`,
        );

        // When Query "SELECT * FROM <table> ORDER BY col" is executed
        const { rows } = await executeAsync(connection, `SELECT * FROM ${tableName} ORDER BY COL`);

        // Then Result should contain 50000 sequentially increasing timestamps from 2024-01-01 00:00:00 UTC
        expect(rows).toHaveLength(rowCount);
        expectLtzDates(
          [rows[0].COL, rows[rowCount - 1].COL],
          [
            dateAtUtc('2024-01-01T00:00:00'),
            new Date(dateAtUtc('2024-01-01T00:00:00').getTime() + (rowCount - 1) * 1000),
          ],
        );
      });
    });
  });

  describe('fetchAsString', () => {
    it('should render timestamp_ltz as YYYY-MM-DD HH24:MI:SS.FF3 TZHTZM', async () => {
      const { rows } = await executeAsync(
        connection,
        `SELECT '2024-01-15 10:30:00 +00:00'::TIMESTAMP_LTZ`,
        { fetchAsString: ['Date'] },
      );
      expect(Object.values(rows[0])).toEqual(['2024-01-15 05:30:00.000 -0500']);
    });

    it("should render a NULL TIMESTAMP_LTZ cell as the string 'NULL'", async () => {
      const { rows } = await executeAsync(connection, `SELECT NULL::TIMESTAMP_LTZ`, {
        fetchAsString: ['Date'],
      });
      expect(Object.values(rows[0])).toEqual(['NULL']);
    });

    it('should render a NULL TIMESTAMP_LTZ cell as null when representNullAsStringNull is disabled', async () => {
      const nullPreservingConnection = await createLiveNullPreservingConnection();
      const { rows } = await executeAsync(nullPreservingConnection, `SELECT NULL::TIMESTAMP_LTZ`, {
        fetchAsString: ['Date'],
      });
      expect(Object.values(rows[0])).toEqual([null]);
    });
  });

  describe('TIMESTAMP_LTZ_OUTPUT_FORMAT and TIMESTAMP_OUTPUT_FORMAT', () => {
    async function selectLtz(
      literal: string,
      options?: Parameters<typeof executeAsync>[2],
    ): Promise<unknown> {
      const { rows } = await executeAsync(
        connection,
        `SELECT '${literal}'::TIMESTAMP_LTZ`,
        options,
      );
      return Object.values(rows[0])[0];
    }

    // statement-level is a known bug in both drivers documented in BCR_LOG.md
    it.each([
      {
        name: 'LTZ only',
        parameters: { TIMESTAMP_LTZ_OUTPUT_FORMAT: 'YYYY-MM-DD HH24:MI:SS.FF6 TZH:TZM' },
      },
      { name: 'generic only', parameters: { TIMESTAMP_OUTPUT_FORMAT: 'YYYY-MM-DD HH24:MI' } },
      {
        name: 'both',
        parameters: {
          TIMESTAMP_LTZ_OUTPUT_FORMAT: 'YYYY-MM-DD HH24:MI:SS.FF6 TZH:TZM',
          TIMESTAMP_OUTPUT_FORMAT: 'YYYY-MM-DD HH24:MI',
        },
      },
    ])('should ignore statement-level $name output format', async ({ parameters }) => {
      expect(
        await selectLtz('2024-01-15 10:30:00 +00:00', { parameters, fetchAsString: ['Date'] }),
      ).toBe('2024-01-15 05:30:00.000 -0500');

      const date = (await selectLtz('2024-01-15 10:30:00 +00:00', {
        parameters,
      })) as SnowflakeDate;
      expect(date.toJSON()).toBe('2024-01-15 05:30:00.000 -0500');
      expect(date.getFormat()).toBe('YYYY-MM-DD HH24:MI:SS.FF3 TZHTZM');
    });

    describe('session-level', () => {
      it('should honor TIMESTAMP_OUTPUT_FORMAT when TIMESTAMP_LTZ_OUTPUT_FORMAT is empty', async () => {
        await setSessionParameterForTest(connection, 'TIMESTAMP_LTZ_OUTPUT_FORMAT', '');
        await setSessionParameterForTest(
          connection,
          'TIMESTAMP_OUTPUT_FORMAT',
          'YYYY-MM-DD HH24:MI TZH:TZM',
        );

        expect(await selectLtz('2024-01-15 10:30:00 +00:00', { fetchAsString: ['Date'] })).toBe(
          '2024-01-15 05:30 -05:00',
        );

        const date = (await selectLtz('2024-01-15 10:30:00 +00:00')) as SnowflakeDate;
        expect(date.toJSON()).toBe('2024-01-15 05:30 -05:00');
        expect(date.getFormat()).toBe('YYYY-MM-DD HH24:MI TZH:TZM');
      });

      it('should honor TIMESTAMP_LTZ_OUTPUT_FORMAT', async () => {
        await setSessionParameterForTest(
          connection,
          'TIMESTAMP_LTZ_OUTPUT_FORMAT',
          'YYYY-MM-DD HH24:MI:SS.FF6 TZH:TZM',
        );

        expect(await selectLtz('2024-01-15 10:30:00 +00:00', { fetchAsString: ['Date'] })).toBe(
          '2024-01-15 05:30:00.000000 -05:00',
        );

        const date = (await selectLtz('2024-01-15 10:30:00 +00:00')) as SnowflakeDate;
        expect(date.toJSON()).toBe('2024-01-15 05:30:00.000000 -05:00');
        expect(date.getFormat()).toBe('YYYY-MM-DD HH24:MI:SS.FF6 TZH:TZM');
      });

      it('should prefer TIMESTAMP_LTZ_OUTPUT_FORMAT when both timestamp output formats are set', async () => {
        await setSessionParameterForTest(
          connection,
          'TIMESTAMP_OUTPUT_FORMAT',
          'YYYY-MM-DD HH24:MI',
        );
        await setSessionParameterForTest(
          connection,
          'TIMESTAMP_LTZ_OUTPUT_FORMAT',
          'YYYY-MM-DD HH24:MI:SS.FF6 TZH:TZM',
        );

        expect(await selectLtz('2024-01-15 10:30:00 +00:00', { fetchAsString: ['Date'] })).toBe(
          '2024-01-15 05:30:00.000000 -05:00',
        );

        const date = (await selectLtz('2024-01-15 10:30:00 +00:00')) as SnowflakeDate;
        expect(date.toJSON()).toBe('2024-01-15 05:30:00.000000 -05:00');
        expect(date.getFormat()).toBe('YYYY-MM-DD HH24:MI:SS.FF6 TZH:TZM');
      });
    });
  });

  it('should expose SnowflakeDate custom methods', async () => {
    const { rows } = await executeAsync(
      connection,
      `SELECT '2024-01-15 10:30:00.123456789 +00:00'::TIMESTAMP_LTZ AS VAL`,
    );
    const date = rows[0].VAL as SnowflakeDate;
    expect(date).toBeInstanceOf(Date);
    expect(date.getEpochSeconds()).toBe(Date.UTC(2024, 0, 15, 10, 30, 0) / 1000);
    expect(date.getNanoSeconds()).toBe(123456789);
    expect(date.getScale()).toBe(9);
    expect(date.getTimezone()).toBe(SESSION_TIMEZONE);
    expect(date.getFormat()).toBe('YYYY-MM-DD HH24:MI:SS.FF3 TZHTZM');
    expect(date.toJSON()).toBe('2024-01-15 05:30:00.123 -0500');
  });
});
