import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import type { Connection, SnowflakeDate } from '../../../types/sdk-types.js';
import { createLiveConnection, createTemporaryTable } from '../../utils/fixtures.js';
import {
  destroyConnectionAsync,
  executeAsync,
  NOT_IMPLEMENTED_IN_NEW_DRIVER,
} from '../../utils/index.js';
import { setSessionParameter, unsetSessionParameter } from '../../utils/query.js';
import { createLiveNullPreservingConnection } from '../utils.js';

function dateAtUtcMidnight(dateLiteral: string): Date {
  return new Date(`${dateLiteral}T00:00:00.000Z`);
}

describe('DATE data type', () => {
  let connection: Connection;

  beforeAll(async () => {
    connection = await createLiveConnection({}, false);
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  describe('tests/definitions/shared/types/date.feature', () => {
    it('should cast date values to appropriate type', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT '2024-01-15'::DATE, '1970-01-01'::DATE, '1999-12-31'::DATE" is executed
      const { statement, rows } = await executeAsync(
        connection,
        `SELECT '2024-01-15'::DATE, '1970-01-01'::DATE, '1999-12-31'::DATE`,
      );

      for (const column of statement.getColumns()!) {
        // Then All values should be returned as DATE type
        expect(column.getType()).toBe('date');
        expect(column.isDate()).toBe(true);
      }
      // And No precision loss should occur
      expect(Object.values(rows[0])).toEqual([
        dateAtUtcMidnight('2024-01-15'),
        dateAtUtcMidnight('1970-01-01'),
        dateAtUtcMidnight('1999-12-31'),
      ]);
    });

    it('should select date literals', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT '2024-01-15'::DATE, '1970-01-01'::DATE, '1999-12-31'::DATE" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT '2024-01-15'::DATE, '1970-01-01'::DATE, '1999-12-31'::DATE`,
      );

      // Then Result should contain dates [2024-01-15, 1970-01-01, 1999-12-31]
      expect(Object.values(rows[0])).toEqual([
        dateAtUtcMidnight('2024-01-15'),
        dateAtUtcMidnight('1970-01-01'),
        dateAtUtcMidnight('1999-12-31'),
      ]);
    });

    it('should select epoch and pre-epoch dates', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT '1970-01-01'::DATE, '1969-12-31'::DATE, '1900-01-01'::DATE" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT '1970-01-01'::DATE, '1969-12-31'::DATE, '1900-01-01'::DATE`,
      );

      // Then Result should contain dates [1970-01-01, 1969-12-31, 1900-01-01]
      expect(Object.values(rows[0])).toEqual([
        dateAtUtcMidnight('1970-01-01'),
        dateAtUtcMidnight('1969-12-31'),
        dateAtUtcMidnight('1900-01-01'),
      ]);
    });

    it('should select historical and boundary dates', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT '0001-01-01'::DATE, '1582-10-15'::DATE, '9999-12-31'::DATE" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT '0001-01-01'::DATE, '1582-10-15'::DATE, '9999-12-31'::DATE`,
      );

      // Then Result should contain dates [0001-01-01, 1582-10-15, 9999-12-31]
      expect(Object.values(rows[0])).toEqual([
        dateAtUtcMidnight('0001-01-01'),
        dateAtUtcMidnight('1582-10-15'),
        dateAtUtcMidnight('9999-12-31'),
      ]);
    });

    it('should handle NULL values for date', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT NULL::DATE, '2024-01-15'::DATE, NULL::DATE" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT NULL::DATE, '2024-01-15'::DATE, NULL::DATE AS NULL_DATE_2`,
      );

      // Then Result should contain [NULL, 2024-01-15, NULL]
      expect(Object.values(rows[0])).toEqual([null, dateAtUtcMidnight('2024-01-15'), null]);
    });

    describe('table operations', () => {
      it('should select dates from table', async () => {
        // Given Snowflake client is logged in
        void connection;

        // And Table with DATE column exists with values ['2024-01-15', '1970-01-01', '1999-12-31']
        const tableName = await createTemporaryTable(connection, 'COL DATE');
        await executeAsync(
          connection,
          `INSERT INTO ${tableName} (COL) VALUES ('2024-01-15'), ('1970-01-01'), ('1999-12-31')`,
        );

        // When Query "SELECT * FROM <table> ORDER BY col" is executed
        const { rows } = await executeAsync(
          connection,
          `SELECT COL FROM ${tableName} ORDER BY COL`,
        );

        // Then Result should contain dates [1970-01-01, 1999-12-31, 2024-01-15]
        expect(rows.map((row) => row.COL)).toEqual([
          dateAtUtcMidnight('1970-01-01'),
          dateAtUtcMidnight('1999-12-31'),
          dateAtUtcMidnight('2024-01-15'),
        ]);
      });

      it('should select dates with NULL from table', async () => {
        // Given Snowflake client is logged in
        void connection;

        // And Table with DATE column exists with values ['2024-01-15', NULL, '1999-12-31']
        const tableName = await createTemporaryTable(connection, 'COL DATE');
        await executeAsync(
          connection,
          `INSERT INTO ${tableName} (COL) VALUES ('2024-01-15'), (NULL), ('1999-12-31')`,
        );

        // When Query "SELECT * FROM <table> ORDER BY col" is executed
        const { rows } = await executeAsync(
          connection,
          `SELECT COL FROM ${tableName} ORDER BY COL`,
        );

        // Then Result should contain [1999-12-31, 2024-01-15, NULL]
        expect(rows.map((row) => row.COL)).toEqual([
          dateAtUtcMidnight('1999-12-31'),
          dateAtUtcMidnight('2024-01-15'),
          null,
        ]);
      });

      it('should select historical and boundary dates from table', async () => {
        // Given Snowflake client is logged in
        void connection;

        // And Table with DATE column exists with values ['0001-01-01', '0100-03-01', '1582-10-15', '9999-12-31']
        const tableName = await createTemporaryTable(connection, 'COL DATE');
        await executeAsync(
          connection,
          `INSERT INTO ${tableName} (COL) VALUES ('0001-01-01'), ('0100-03-01'), ('1582-10-15'), ('9999-12-31')`,
        );

        // When Query "SELECT * FROM <table> ORDER BY col" is executed
        const { rows } = await executeAsync(
          connection,
          `SELECT COL FROM ${tableName} ORDER BY COL`,
        );

        // Then Result should contain dates [0001-01-01, 0100-03-01, 1582-10-15, 9999-12-31]
        expect(rows.map((row) => row.COL)).toEqual([
          dateAtUtcMidnight('0001-01-01'),
          dateAtUtcMidnight('0100-03-01'),
          dateAtUtcMidnight('1582-10-15'),
          dateAtUtcMidnight('9999-12-31'),
        ]);
      });
    });

    describe.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)('multiple chunks', () => {
      const ONE_DAY_MS = 1000 * 60 * 60 * 24;

      it('should download large result set for date', async () => {
        const rowCount = 100_000;

        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT DATEADD(day, ROW_NUMBER() OVER (ORDER BY seq4()) - 1, '1970-01-01'::DATE) as d FROM TABLE(GENERATOR(ROWCOUNT => 100000)) ORDER BY d" is executed
        const { rows } = await executeAsync(
          connection,
          `SELECT DATEADD(day, ROW_NUMBER() OVER (ORDER BY seq4()) - 1, '1970-01-01'::DATE) AS D FROM TABLE(GENERATOR(ROWCOUNT => ${rowCount})) ORDER BY D`,
        );

        // Then Result should contain 100000 rows with sequential dates starting from 1970-01-01
        expect(rows).toHaveLength(rowCount);
        expect(rows[0].D).toEqual(dateAtUtcMidnight('1970-01-01'));
        expect(rows[rowCount - 1].D).toEqual(
          new Date(dateAtUtcMidnight('1970-01-01').getTime() + (rowCount - 1) * ONE_DAY_MS),
        );
      });

      it('should download large result set for date from table', async () => {
        const rowCount = 100_000;

        // Given Snowflake client is logged in
        void connection;

        // And Table with DATE column exists with 100000 sequential dates starting from 1970-01-01
        const tableName = await createTemporaryTable(connection, 'COL DATE');
        await executeAsync(
          connection,
          `INSERT INTO ${tableName} (COL)
             SELECT DATEADD(day, ROW_NUMBER() OVER (ORDER BY seq4()) - 1, '1970-01-01'::DATE)
             FROM TABLE(GENERATOR(ROWCOUNT => ${rowCount}))`,
        );

        // When Query "SELECT * FROM <table> ORDER BY col" is executed
        const { rows } = await executeAsync(
          connection,
          `SELECT COL FROM ${tableName} ORDER BY COL`,
        );

        // Then Result should contain 100000 rows with sequential dates starting from 1970-01-01
        expect(rows).toHaveLength(rowCount);
        expect(rows[0].COL).toEqual(dateAtUtcMidnight('1970-01-01'));
        expect(rows[rowCount - 1].COL).toEqual(
          new Date(dateAtUtcMidnight('1970-01-01').getTime() + (rowCount - 1) * ONE_DAY_MS),
        );
      });
    });

    describe('parameter binding', () => {
      it('should select date using parameter binding', async () => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT ?::DATE, ?::DATE, ?::DATE" is executed with bound date values [2024-01-15, 1970-01-01, 1999-12-31]
        const { rows } = await executeAsync(
          connection,
          `SELECT ?::DATE AS COL1, ?::DATE AS COL2, ?::DATE AS COL3`,
          { binds: ['2024-01-15', '1970-01-01', '1999-12-31'] },
        );

        // Then Result should contain [2024-01-15, 1970-01-01, 1999-12-31]
        expect(Object.values(rows[0])).toEqual([
          dateAtUtcMidnight('2024-01-15'),
          dateAtUtcMidnight('1970-01-01'),
          dateAtUtcMidnight('1999-12-31'),
        ]);
      });

      it('should select null date using parameter binding', async () => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT ?::DATE" is executed with bound NULL value
        const { rows } = await executeAsync(connection, `SELECT ?::DATE`, {
          binds: [null],
        });

        // Then Result should contain [NULL]
        expect(Object.values(rows[0])).toEqual([null]);
      });

      it('should insert date using parameter binding', async () => {
        // Given Snowflake client is logged in
        void connection;

        // And Table with DATE column exists
        const tableName = await createTemporaryTable(connection, 'COL DATE');
        // When Date values [2024-01-15, 1970-01-01, 1999-12-31] are inserted using parameter binding
        await executeAsync(connection, `INSERT INTO ${tableName} (COL) VALUES (?)`, {
          binds: [['2024-01-15'], ['1970-01-01'], ['1999-12-31']],
        });

        // And Query "SELECT * FROM <table> ORDER BY col" is executed
        const { rows } = await executeAsync(
          connection,
          `SELECT COL FROM ${tableName} ORDER BY COL`,
        );

        // Then Result should contain dates [1970-01-01, 1999-12-31, 2024-01-15]
        expect(rows.map((row) => row.COL)).toEqual([
          dateAtUtcMidnight('1970-01-01'),
          dateAtUtcMidnight('1999-12-31'),
          dateAtUtcMidnight('2024-01-15'),
        ]);
      });
    });
  });

  describe('fetchAsString', () => {
    it('should render dates as YYYY-MM-DD', async () => {
      const { rows } = await executeAsync(
        connection,
        `SELECT '2024-01-15'::DATE, '1900-01-01'::DATE, '9999-12-31'::DATE`,
        { fetchAsString: ['Date'] },
      );
      expect(Object.values(rows[0])).toEqual(['2024-01-15', '1900-01-01', '9999-12-31']);
    });

    it("should render a NULL DATE cell as the string 'NULL'", async () => {
      const { rows } = await executeAsync(connection, `SELECT NULL::DATE`, {
        fetchAsString: ['Date'],
      });
      expect(Object.values(rows[0])).toEqual(['NULL']);
    });

    it('should render a NULL DATE cell as null when representNullAsStringNull is disabled', async () => {
      const nullPreservingConnection = await createLiveNullPreservingConnection();
      const { rows } = await executeAsync(nullPreservingConnection, `SELECT NULL::DATE`, {
        fetchAsString: ['Date'],
      });
      expect(Object.values(rows[0])).toEqual([null]);
    });
  });

  describe('DATE_OUTPUT_FORMAT', () => {
    // statement-level is a known bug in both drivers documented in BCR_LOG.md
    it('should ignore DATE_OUTPUT_FORMAT when set at statement-level for fetchAsString', async () => {
      const { rows } = await executeAsync(connection, `SELECT '2024-01-15'::DATE`, {
        parameters: { DATE_OUTPUT_FORMAT: 'DD-MON-YYYY' },
        fetchAsString: ['Date'],
      });
      expect(Object.values(rows[0])).toEqual(['2024-01-15']);
    });

    it.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)(
      'should ignore DATE_OUTPUT_FORMAT when set at statement-level for toJSON and getFormat',
      async () => {
        const { rows } = await executeAsync(connection, `SELECT '2024-01-15'::DATE AS VAL`, {
          parameters: { DATE_OUTPUT_FORMAT: 'DD-MON-YYYY' },
        });
        const date = rows[0].VAL as SnowflakeDate;
        expect(date.toJSON()).toBe('2024-01-15');
        expect(date.getFormat()).toBe('YYYY-MM-DD');
      },
    );

    describe.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)(
      'when DATE_OUTPUT_FORMAT is set on the session',
      () => {
        beforeAll(async () => {
          await setSessionParameter(connection, 'DATE_OUTPUT_FORMAT', 'DD-MON-YYYY');
        });

        afterAll(async () => {
          await unsetSessionParameter(connection, 'DATE_OUTPUT_FORMAT');
        });

        it('should honor DATE_OUTPUT_FORMAT for fetchAsString', async () => {
          const { rows } = await executeAsync(connection, `SELECT '2024-01-15'::DATE as VAL`, {
            fetchAsString: ['Date'],
          });
          expect(rows[0].VAL).toBe('15-Jan-2024');
        });

        it('should honor DATE_OUTPUT_FORMAT for toJSON and getFormat', async () => {
          const { rows } = await executeAsync(connection, `SELECT '2024-01-15'::DATE as VAL`);
          const date = rows[0].VAL as SnowflakeDate;
          expect(date.toJSON()).toBe('15-Jan-2024');
          expect(date.getFormat()).toBe('DD-MON-YYYY');
        });
      },
    );
  });

  it.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)(
    'should expose SnowflakeDate custom methods',
    async () => {
      const { rows } = await executeAsync(connection, `SELECT '2024-01-15'::DATE AS VAL`);
      const date = rows[0].VAL as SnowflakeDate;
      expect(date).toBeInstanceOf(Date);
      expect(date.getEpochSeconds()).toBe(Date.UTC(2024, 0, 15) / 1000);
      expect(date.getNanoSeconds()).toBe(0);
      expect(date.getScale()).toBe(0);
      expect(date.getTimezone()).toBe('UTC');
      expect(date.toJSON()).toBe('2024-01-15');
    },
  );
});
