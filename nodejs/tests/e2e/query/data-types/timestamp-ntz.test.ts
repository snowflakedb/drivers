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

type ExpectedNtzValue = Date | null | { date: Date | null; nanoSeconds?: number };

function expectNtzDates(values: unknown[], expected: ExpectedNtzValue[]): void {
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
    expect(actual.getTimezone()).toBe('UTC');
    if (nanoSeconds !== undefined) {
      expect(actual.getNanoSeconds()).toBe(nanoSeconds);
    }
  });
}

describe.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)('TIMESTAMP_NTZ data type', () => {
  let connection: Connection;

  beforeAll(async () => {
    connection = await createLiveConnection({}, false);
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  describe('tests/definitions/shared/types/timestamp_ntz.feature', () => {
    it('should cast timestamp_ntz values to appropriate type', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT '2024-01-15 10:30:00'::TIMESTAMP_NTZ" is executed
      const { statement, rows } = await executeAsync(
        connection,
        `SELECT '2024-01-15 10:30:00'::TIMESTAMP_NTZ`,
      );
      const column = getStatementColumn(statement, 0);
      const value = Object.values(rows[0])[0];

      // Then All values should be returned as appropriate type
      expect(column.getType()).toBe('timestamp_ntz');
      expect(column.isTimestamp()).toBe(true);
      expect(column.isTimestampNtz()).toBe(true);

      // And Values should not have timezone info
      expectNtzDates([value], [dateAtUtc('2024-01-15T10:30:00')]);
    });

    it.each<{ values: string; query: string; expected: ExpectedNtzValue[] }>([
      {
        values: 'basic',
        query: `'2024-01-15 10:30:00'::TIMESTAMP_NTZ, '2024-06-20 14:45:30'::TIMESTAMP_NTZ`,
        expected: [dateAtUtc('2024-01-15T10:30:00'), dateAtUtc('2024-06-20T14:45:30')],
      },
      {
        values: 'epoch',
        query: `'1970-01-01 00:00:00'::TIMESTAMP_NTZ`,
        expected: [dateAtUtc('1970-01-01T00:00:00')],
      },
      {
        values: 'microseconds',
        query: `'2024-01-15 10:30:00.123456'::TIMESTAMP_NTZ`,
        expected: [{ date: dateAtUtc('2024-01-15T10:30:00.123'), nanoSeconds: 123456000 }],
      },
    ])('should select timestamp_ntz $values', async ({ query, expected }) => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT <query_values>" is executed
      const { rows } = await executeAsync(connection, `SELECT ${query}`);

      // Then Result should contain timestamps <expected_values>
      void 0;
      // And Values should not have timezone info
      expectNtzDates(Object.values(rows[0]), expected);
    });

    it('should select sub-second timestamp_ntz values before epoch', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Sub-second timestamp_ntz values before the epoch are selected
      const { rows } = await executeAsync(
        connection,
        `SELECT '1969-12-31 23:59:59.999999999'::TIMESTAMP_NTZ AS NEAR_EPOCH,
                '1969-12-31 23:59:58.5'::TIMESTAMP_NTZ(3) AS SCALE_3`,
      );

      // Then Result should contain the expected sub-second values before the epoch
      expectNtzDates(
        [rows[0].NEAR_EPOCH, rows[0].SCALE_3],
        [
          { date: dateAtUtc('1969-12-31T23:59:59.999'), nanoSeconds: 999999999 },
          { date: dateAtUtc('1969-12-31T23:59:58.500'), nanoSeconds: 500000000 },
        ],
      );
    });

    it('should handle NULL values for timestamp_ntz', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT '2024-01-15 10:30:00'::TIMESTAMP_NTZ, NULL::TIMESTAMP_NTZ" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT '2024-01-15 10:30:00'::TIMESTAMP_NTZ, NULL::TIMESTAMP_NTZ`,
      );

      // Then Result should contain [2024-01-15 10:30:00, NULL]
      expectNtzDates(Object.values(rows[0]), [dateAtUtc('2024-01-15T10:30:00'), null]);
    });

    it.each<{ values: string; insert: string; expected: ExpectedNtzValue[] }>([
      {
        values: 'basic',
        insert: `('2024-01-15 10:30:00'), ('2024-06-20 14:45:30')`,
        expected: [dateAtUtc('2024-01-15T10:30:00'), dateAtUtc('2024-06-20T14:45:30')],
      },
      {
        values: 'epoch',
        insert: `('1970-01-01 00:00:00'), ('2024-01-15 10:30:00')`,
        expected: [dateAtUtc('1970-01-01T00:00:00'), dateAtUtc('2024-01-15T10:30:00')],
      },
      {
        values: 'microseconds',
        insert: `('2024-01-15 10:30:00'), ('2024-01-15 10:30:00.123456')`,
        expected: [
          dateAtUtc('2024-01-15T10:30:00'),
          { date: dateAtUtc('2024-01-15T10:30:00.123'), nanoSeconds: 123456000 },
        ],
      },
      {
        values: 'null',
        insert: `(NULL), ('2024-01-15 10:30:00')`,
        expected: [dateAtUtc('2024-01-15T10:30:00'), null],
      },
    ])('should select $values from table for timestamp_ntz', async ({ insert, expected }) => {
      // Given Snowflake client is logged in
      void connection;

      // And Table with TIMESTAMP_NTZ column exists with values <insert_values>
      const tableName = await createTemporaryTable(connection, 'COL TIMESTAMP_NTZ');
      await executeAsync(connection, `INSERT INTO ${tableName} (COL) VALUES ${insert}`);

      // When Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT * FROM ${tableName} ORDER BY COL NULLS LAST`,
      );

      // Then Result should contain timestamps <expected_values>
      expectNtzDates(
        rows.map((row) => row.COL),
        expected,
      );
    });

    describe('parameter binding', () => {
      it('should select timestamp_ntz using parameter binding', async () => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT ?::TIMESTAMP_NTZ, ?::TIMESTAMP_NTZ" is executed with bound timestamp values
        const { rows } = await executeAsync(
          connection,
          `SELECT ?::TIMESTAMP_NTZ AS COL1, ?::TIMESTAMP_NTZ AS COL2`,
          { binds: ['2024-01-15 10:30:00', '2024-06-20 14:45:30'] },
        );

        // Then Result should contain [2024-01-15 10:30:00, 2024-06-20 14:45:30]
        void 0;
        // And Values should not have timezone info
        expectNtzDates(Object.values(rows[0]), [
          dateAtUtc('2024-01-15T10:30:00'),
          dateAtUtc('2024-06-20T14:45:30'),
        ]);
      });

      it('should return NULL when selecting timestamp_ntz using parameter binding with NULL value', async () => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT ?::TIMESTAMP_NTZ" is executed with bound NULL value
        const { rows } = await executeAsync(connection, `SELECT ?::TIMESTAMP_NTZ`, {
          binds: [null],
        });

        // Then Result should contain [NULL]
        expectNtzDates(Object.values(rows[0]), [null]);
      });

      it('should insert timestamp_ntz using parameter binding', async () => {
        // Given Snowflake client is logged in
        void connection;

        // And Table with TIMESTAMP_NTZ column exists
        const tableName = await createTemporaryTable(connection, 'COL TIMESTAMP_NTZ');

        // When Timestamp values are bulk-inserted using multirow binding
        await executeAsync(connection, `INSERT INTO ${tableName} (COL) VALUES (?)`, {
          binds: [['2024-01-15 10:30:00'], ['2024-06-20 14:45:30'], [null]],
        });

        // And Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
        const { rows } = await executeAsync(
          connection,
          `SELECT * FROM ${tableName} ORDER BY COL NULLS LAST`,
        );

        // Then SELECT should return the inserted values in ascending order
        expectNtzDates(
          rows.map((row) => row.COL),
          [dateAtUtc('2024-01-15T10:30:00'), dateAtUtc('2024-06-20T14:45:30'), null],
        );
      });
    });

    describe('type mapping aliases', () => {
      it.each(['TIMESTAMP', 'DATETIME'])(
        'should return naive datetime for $typeName alias when session mapping is TIMESTAMP_NTZ',
        async (typeName) => {
          // Given Snowflake client is logged in
          void connection;

          // And Session TIMESTAMP_TYPE_MAPPING is set to TIMESTAMP_NTZ
          await setSessionParameterForTest(connection, 'TIMESTAMP_TYPE_MAPPING', 'TIMESTAMP_NTZ');

          // When Query "SELECT '2024-01-15 10:30:00'::<type_name>" is executed
          const { statement, rows } = await executeAsync(
            connection,
            `SELECT '2024-01-15 10:30:00'::${typeName}`,
          );
          const column = getStatementColumn(statement, 0);
          const value = Object.values(rows[0])[0];

          // Then All values should be returned as appropriate type
          expect(column.getType()).toBe('timestamp_ntz');
          expect(column.isTimestampNtz()).toBe(true);

          // And Values should not have timezone info
          expectNtzDates([value], [dateAtUtc('2024-01-15T10:30:00')]);
        },
      );

      it('should return aware datetime for TIMESTAMP alias when session mapping is TIMESTAMP_LTZ', async () => {
        // Given Snowflake client is logged in
        void connection;

        // And Session TIMESTAMP_TYPE_MAPPING is set to TIMESTAMP_LTZ
        await setSessionParameterForTest(connection, 'TIMESTAMP_TYPE_MAPPING', 'TIMESTAMP_LTZ');
        await setSessionParameterForTest(connection, 'TIMEZONE', 'America/New_York');

        // When Query "SELECT '2024-01-15 10:30:00'::TIMESTAMP" is executed
        const { statement, rows } = await executeAsync(
          connection,
          `SELECT '2024-01-15 10:30:00'::TIMESTAMP`,
        );
        const column = getStatementColumn(statement, 0);
        const value = Object.values(rows[0])[0] as SnowflakeDate;

        // Then All values should be returned as appropriate type
        expect(column.getType()).toBe('timestamp_ltz');
        expect(column.isTimestampLtz()).toBe(true);
        expect(value).toBeInstanceOf(Date);

        // And Values should have timezone info
        expect(value.getTimezone()).toBe('America/New_York');
      });
    });

    describe.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)('multiple chunks', () => {
      const rowCount = 50_000;

      it('should download large result set with multiple chunks for timestamp_ntz', async () => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1, '2024-01-01 00:00:00'::TIMESTAMP_NTZ) as ts FROM TABLE(GENERATOR(ROWCOUNT => 50000)) ORDER BY ts" is executed
        const { rows } = await executeAsync(
          connection,
          `SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1, '2024-01-01 00:00:00'::TIMESTAMP_NTZ) AS TS FROM TABLE(GENERATOR(ROWCOUNT => ${rowCount})) ORDER BY TS`,
        );

        // Then Result should contain 50000 sequentially increasing timestamps from 2024-01-01 00:00:00
        expect(rows).toHaveLength(rowCount);
        expectNtzDates(
          [rows[0].TS, rows[rowCount - 1].TS],
          [
            dateAtUtc('2024-01-01T00:00:00'),
            new Date(dateAtUtc('2024-01-01T00:00:00').getTime() + (rowCount - 1) * 1000),
          ],
        );
      });

      it('should download large result set with multiple chunks from table for timestamp_ntz', async () => {
        // Given Snowflake client is logged in
        void connection;

        // And Table with TIMESTAMP_NTZ column exists with 50000 sequential timestamp values
        const tableName = await createTemporaryTable(connection, 'COL TIMESTAMP_NTZ');
        await executeAsync(
          connection,
          `INSERT INTO ${tableName} (COL)
             SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1, '2024-01-01 00:00:00'::TIMESTAMP_NTZ)
             FROM TABLE(GENERATOR(ROWCOUNT => ${rowCount}))`,
        );

        // When Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
        const { rows } = await executeAsync(
          connection,
          `SELECT * FROM ${tableName} ORDER BY COL NULLS LAST`,
        );

        // Then Result should contain 50000 sequentially increasing timestamps from 2024-01-01 00:00:00
        expect(rows).toHaveLength(rowCount);
        expectNtzDates(
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
    it('should render timestamp_ntz as YYYY-MM-DD HH24:MI:SS.FF3', async () => {
      const { rows } = await executeAsync(
        connection,
        `SELECT '2024-01-15 10:30:00'::TIMESTAMP_NTZ`,
        { fetchAsString: ['Date'] },
      );
      expect(Object.values(rows[0])).toEqual(['2024-01-15 10:30:00.000']);
    });

    it("should render a NULL TIMESTAMP_NTZ cell as the string 'NULL'", async () => {
      const { rows } = await executeAsync(connection, `SELECT NULL::TIMESTAMP_NTZ`, {
        fetchAsString: ['Date'],
      });
      expect(Object.values(rows[0])).toEqual(['NULL']);
    });

    it('should render a NULL TIMESTAMP_NTZ cell as null when representNullAsStringNull is disabled', async () => {
      const nullPreservingConnection = await createLiveNullPreservingConnection();
      const { rows } = await executeAsync(nullPreservingConnection, `SELECT NULL::TIMESTAMP_NTZ`, {
        fetchAsString: ['Date'],
      });
      expect(Object.values(rows[0])).toEqual([null]);
    });
  });

  describe('TIMESTAMP_NTZ_OUTPUT_FORMAT and TIMESTAMP_OUTPUT_FORMAT', () => {
    async function selectNtz(
      literal: string,
      options?: Parameters<typeof executeAsync>[2],
    ): Promise<unknown> {
      const { rows } = await executeAsync(
        connection,
        `SELECT '${literal}'::TIMESTAMP_NTZ`,
        options,
      );
      return Object.values(rows[0])[0];
    }

    // statement-level is a known bug in both drivers documented in BCR_LOG.md
    it.each([
      {
        name: 'NTZ only',
        parameters: { TIMESTAMP_NTZ_OUTPUT_FORMAT: 'YYYY-MM-DD HH24:MI:SS.FF6' },
      },
      { name: 'generic only', parameters: { TIMESTAMP_OUTPUT_FORMAT: 'YYYY-MM-DD HH24:MI' } },
      {
        name: 'both',
        parameters: {
          TIMESTAMP_NTZ_OUTPUT_FORMAT: 'YYYY-MM-DD HH24:MI:SS.FF6',
          TIMESTAMP_OUTPUT_FORMAT: 'YYYY-MM-DD HH24:MI',
        },
      },
    ])('should ignore statement-level $name output format', async ({ parameters }) => {
      expect(await selectNtz('2024-01-15 10:30:00', { parameters, fetchAsString: ['Date'] })).toBe(
        '2024-01-15 10:30:00.000',
      );

      const date = (await selectNtz('2024-01-15 10:30:00', { parameters })) as SnowflakeDate;
      expect(date.toJSON()).toBe('2024-01-15 10:30:00.000');
      expect(date.getFormat()).toBe('YYYY-MM-DD HH24:MI:SS.FF3');
    });

    describe('session-level', () => {
      it('should honor TIMESTAMP_OUTPUT_FORMAT when TIMESTAMP_NTZ_OUTPUT_FORMAT is empty', async () => {
        await setSessionParameterForTest(connection, 'TIMESTAMP_NTZ_OUTPUT_FORMAT', '');
        await setSessionParameterForTest(
          connection,
          'TIMESTAMP_OUTPUT_FORMAT',
          'YYYY-MM-DD HH24:MI',
        );

        expect(await selectNtz('2024-01-15 10:30:00', { fetchAsString: ['Date'] })).toBe(
          '2024-01-15 10:30',
        );

        const date = (await selectNtz('2024-01-15 10:30:00')) as SnowflakeDate;
        expect(date.toJSON()).toBe('2024-01-15 10:30');
        expect(date.getFormat()).toBe('YYYY-MM-DD HH24:MI');
      });

      it('should honor TIMESTAMP_NTZ_OUTPUT_FORMAT', async () => {
        await setSessionParameterForTest(
          connection,
          'TIMESTAMP_NTZ_OUTPUT_FORMAT',
          'YYYY-MM-DD HH24:MI:SS.FF6',
        );

        expect(await selectNtz('2024-01-15 10:30:00', { fetchAsString: ['Date'] })).toBe(
          '2024-01-15 10:30:00.000000',
        );

        const date = (await selectNtz('2024-01-15 10:30:00')) as SnowflakeDate;
        expect(date.toJSON()).toBe('2024-01-15 10:30:00.000000');
        expect(date.getFormat()).toBe('YYYY-MM-DD HH24:MI:SS.FF6');
      });

      it('should prefer TIMESTAMP_NTZ_OUTPUT_FORMAT when both timestamp output formats are set', async () => {
        await setSessionParameterForTest(
          connection,
          'TIMESTAMP_OUTPUT_FORMAT',
          'YYYY-MM-DD HH24:MI',
        );
        await setSessionParameterForTest(
          connection,
          'TIMESTAMP_NTZ_OUTPUT_FORMAT',
          'YYYY-MM-DD HH24:MI:SS.FF6',
        );

        expect(await selectNtz('2024-01-15 10:30:00', { fetchAsString: ['Date'] })).toBe(
          '2024-01-15 10:30:00.000000',
        );

        const date = (await selectNtz('2024-01-15 10:30:00')) as SnowflakeDate;
        expect(date.toJSON()).toBe('2024-01-15 10:30:00.000000');
        expect(date.getFormat()).toBe('YYYY-MM-DD HH24:MI:SS.FF6');
      });
    });
  });

  it('should expose SnowflakeDate custom methods', async () => {
    const { rows } = await executeAsync(
      connection,
      `SELECT '2024-01-15 10:30:00.123456789'::TIMESTAMP_NTZ AS VAL`,
    );
    const date = rows[0].VAL as SnowflakeDate;
    expect(date).toBeInstanceOf(Date);
    expect(date.getEpochSeconds()).toBe(Date.UTC(2024, 0, 15, 10, 30, 0) / 1000);
    expect(date.getNanoSeconds()).toBe(123456789);
    expect(date.getScale()).toBe(9);
    expect(date.getTimezone()).toBe('UTC');
    expect(date.getFormat()).toBe('YYYY-MM-DD HH24:MI:SS.FF3');
    expect(date.toJSON()).toBe('2024-01-15 10:30:00.123');
  });
});
