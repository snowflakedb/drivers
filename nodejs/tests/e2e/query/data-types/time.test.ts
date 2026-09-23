import { afterAll, beforeAll, describe, expect, it, onTestFinished } from 'vitest';
import type { Connection } from '../../../types/sdk-types.js';
import { createLiveConnection, createTemporaryTable } from '../../utils/fixtures.js';
import {
  destroyConnectionAsync,
  executeAsync,
  getStatementColumn,
  isRunningNewDriverWithBD,
  NOT_IMPLEMENTED_IN_NEW_DRIVER,
} from '../../utils/index.js';
import { setSessionParameter, unsetSessionParameter } from '../../utils/query.js';
import { createLiveNullPreservingConnection } from '../utils.js';

describe('TIME data type', () => {
  let connection: Connection;

  beforeAll(async () => {
    connection = await createLiveConnection({}, false);
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  async function setTimeOutputFormat(outputFormat: string) {
    await setSessionParameter(connection, 'TIME_OUTPUT_FORMAT', outputFormat);
    onTestFinished(async () => {
      await unsetSessionParameter(connection, 'TIME_OUTPUT_FORMAT');
    });
  }

  describe('tests/definitions/shared/types/time.feature', () => {
    it('should cast time values to appropriate type', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT '10:30:00'::TIME, '00:00:00'::TIME, '23:59:59'::TIME" is executed
      const { statement } = await executeAsync(
        connection,
        `SELECT '10:30:00'::TIME, '00:00:00'::TIME, '23:59:59'::TIME`,
      );

      // Then All values should be returned as appropriate type
      for (const columnIndex of [0, 1, 2]) {
        const column = getStatementColumn(statement, columnIndex);
        expect(column.getType()).toBe('time');
        expect(column.isTime()).toBe(true);
      }
    });

    it.each([
      {
        values: 'basic',
        query: `'10:30:00'::TIME, '14:45:30'::TIME, '23:59:59'::TIME`,
        expected: ['10:30:00', '14:45:30', '23:59:59'],
      },
      { values: 'midnight', query: `'00:00:00'::TIME`, expected: ['00:00:00'] },
      {
        values: 'microseconds',
        query: `'10:30:00.123456'::TIME`,
        outputFormat: 'HH24:MI:SS.FF6',
        expected: ['10:30:00.123456'],
      },
    ])('should select time $values', async ({ query, outputFormat, expected }) => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT <query_values>" is executed
      if (outputFormat) {
        await setTimeOutputFormat(outputFormat);
      }
      const { rows } = await executeAsync(connection, `SELECT ${query}`);

      // Then Result should contain times <expected_values>
      expect(Object.values(rows[0])).toEqual(expected);
    });

    it.each([
      { scale: 0, expected: '10:30:00', oldDriverExpected: '10:30:00.000000000' },
      { scale: 3, expected: '10:30:00.123', oldDriverExpected: '10:30:00.123000000' },
      { scale: 6, expected: '10:30:00.123456', oldDriverExpected: '10:30:00.123456000' },
    ])('should handle time precision $scale', async ({ scale, expected, oldDriverExpected }) => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT '10:30:00.123456789'::TIME(<scale>)" is executed
      await setTimeOutputFormat('HH24:MI:SS.FF9');
      const { rows } = await executeAsync(
        connection,
        `SELECT '10:30:00.123456789'::TIME(${scale})`,
      );

      // Then Result should contain [<expected>]
      if (isRunningNewDriverWithBD('BD#16')) {
        expect(Object.values(rows[0])).toEqual([expected]);
      } else {
        expect(Object.values(rows[0])).toEqual([oldDriverExpected]);
      }
    });

    it('should preserve nanosecond precision for time', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT '10:30:00.123456789'::TIME" is executed
      await setTimeOutputFormat('HH24:MI:SS.FF9');
      const { rows } = await executeAsync(connection, `SELECT '10:30:00.123456789'::TIME`);

      // Then Result should contain [10:30:00.123456789]
      expect(Object.values(rows[0])).toEqual(['10:30:00.123456789']);
    });

    it('should handle NULL values for time', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT '10:30:00'::TIME, NULL::TIME, '23:59:59'::TIME" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT '10:30:00'::TIME, NULL::TIME, '23:59:59'::TIME`,
      );

      // Then Result should contain [10:30:00, NULL, 23:59:59]
      if (isRunningNewDriverWithBD('BD#14')) {
        expect(Object.values(rows[0])).toEqual(['10:30:00', null, '23:59:59']);
      } else {
        expect(Object.values(rows[0])).toEqual(['10:30:00', 'NULL', '23:59:59']);
      }
    });

    it.each([
      {
        values: 'basic',
        insert: `('10:30:00'), ('14:45:30'), ('23:59:59')`,
        expected: ['10:30:00', '14:45:30', '23:59:59'],
      },
      {
        values: 'midnight',
        insert: `('00:00:00'), ('12:00:00'), ('23:59:59')`,
        expected: ['00:00:00', '12:00:00', '23:59:59'],
      },
    ])('should select $values from table for time', async ({ insert, expected }) => {
      // Given Snowflake client is logged in
      void connection;

      // And Table with TIME column exists with values <insert_values>
      const tableName = await createTemporaryTable(connection, 'COL TIME');
      await executeAsync(connection, `INSERT INTO ${tableName} (COL) VALUES ${insert}`);

      // When Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT * FROM ${tableName} ORDER BY COL NULLS LAST`,
      );

      // Then Result should contain times <expected_values>
      expect(rows.map((row) => row.COL)).toEqual(expected);
    });

    it('should select microseconds from table for time', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And Table with TIME column exists with values ['10:30:00', '10:30:00.123456']
      const tableName = await createTemporaryTable(connection, 'COL TIME');
      await executeAsync(
        connection,
        `INSERT INTO ${tableName} (COL) VALUES ('10:30:00'), ('10:30:00.123456')`,
      );

      // When Query "SELECT * FROM {table} ORDER BY col NULLS LAST" is executed
      await setTimeOutputFormat('HH24:MI:SS.FF6');
      const { rows } = await executeAsync(
        connection,
        `SELECT * FROM ${tableName} ORDER BY COL NULLS LAST`,
      );

      // Then Result should contain times [10:30:00, 10:30:00.123456]
      expect(rows.map((row) => row.COL)).toEqual(['10:30:00.000000', '10:30:00.123456']);
    });

    it('should select null from table for time', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And Table with TIME column exists with values [NULL, '10:30:00']
      const tableName = await createTemporaryTable(connection, 'COL TIME');
      await executeAsync(connection, `INSERT INTO ${tableName} (COL) VALUES (NULL), ('10:30:00')`);

      // When Query "SELECT * FROM {table} ORDER BY col NULLS LAST" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT * FROM ${tableName} ORDER BY COL NULLS LAST`,
      );

      // Then Result should contain times [10:30:00, NULL]
      if (isRunningNewDriverWithBD('BD#14')) {
        expect(rows.map((row) => row.COL)).toEqual(['10:30:00', null]);
      } else {
        expect(rows.map((row) => row.COL)).toEqual(['10:30:00', 'NULL']);
      }
    });

    describe('parameter binding', () => {
      it('should select time using parameter binding', async () => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT ?::TIME, ?::TIME, ?::TIME" is executed with bound time values [10:30:00, 14:45:30, 23:59:59]
        const { rows } = await executeAsync(
          connection,
          'SELECT ?::TIME AS COL1, ?::TIME AS COL2, ?::TIME AS COL3',
          { binds: ['10:30:00', '14:45:30', '23:59:59'] },
        );

        // Then Result should contain times [10:30:00, 14:45:30, 23:59:59]
        expect(Object.values(rows[0])).toEqual(['10:30:00', '14:45:30', '23:59:59']);
      });

      it('should select null time using parameter binding', async () => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT ?::TIME" is executed with bound NULL value
        const { rows } = await executeAsync(connection, 'SELECT ?::TIME', {
          binds: [null],
        });

        // Then Result should contain [NULL]
        if (isRunningNewDriverWithBD('BD#14')) {
          expect(Object.values(rows[0])).toEqual([null]);
        } else {
          expect(Object.values(rows[0])).toEqual(['NULL']);
        }
      });

      it('should insert time using parameter binding', async () => {
        // Given Snowflake client is logged in
        void connection;

        // And Table with TIME column exists
        const tableName = await createTemporaryTable(connection, 'COL TIME');

        // When Time values [00:00:00, 10:30:00, 14:45:30, 23:59:59] are inserted using binding
        await executeAsync(connection, `INSERT INTO ${tableName} (COL) VALUES (?)`, {
          binds: [['00:00:00'], ['10:30:00'], ['14:45:30'], ['23:59:59']],
        });

        // And Query "SELECT * FROM <table> ORDER BY col" is executed
        const { rows } = await executeAsync(connection, `SELECT * FROM ${tableName} ORDER BY COL`);

        // Then Result should contain times [00:00:00, 10:30:00, 14:45:30, 23:59:59]
        expect(rows.map((row) => row.COL)).toEqual([
          '00:00:00',
          '10:30:00',
          '14:45:30',
          '23:59:59',
        ]);
      });

      it('should insert time with fractional seconds using parameter binding', async () => {
        // Given Snowflake client is logged in
        void connection;

        // And Table with TIME column exists
        const tableName = await createTemporaryTable(connection, 'COL TIME');
        // When Time values [10:30:00.123456, 14:45:30.654321] are bulk-inserted using multirow binding
        await executeAsync(connection, `INSERT INTO ${tableName} (COL) VALUES (?)`, {
          binds: [['10:30:00.123456'], ['14:45:30.654321']],
        });

        // And Query "SELECT * FROM <table> ORDER BY col" is executed
        await setTimeOutputFormat('HH24:MI:SS.FF6');
        const { rows } = await executeAsync(connection, `SELECT * FROM ${tableName} ORDER BY COL`);

        // Then Result should contain times [10:30:00.123456, 14:45:30.654321]
        expect(rows.map((row) => row.COL)).toEqual(['10:30:00.123456', '14:45:30.654321']);
      });
    });

    describe.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)('multiple chunks', () => {
      const rowCount = 100_000;

      it('should download large result set with multiple chunks for time', async () => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT TIMEADD(millisecond, ROW_NUMBER() OVER (ORDER BY seq4()) - 1, '00:00:00'::TIME) as t FROM TABLE(GENERATOR(ROWCOUNT => 100000)) ORDER BY t" is executed
        const { rows } = await executeAsync(
          connection,
          `SELECT TIMEADD(millisecond, ROW_NUMBER() OVER (ORDER BY seq4()) - 1, '00:00:00'::TIME) as T FROM TABLE(GENERATOR(ROWCOUNT => ${rowCount})) ORDER BY T`,
        );

        // Then Result should contain 100000 sequentially increasing time values from 00:00:00
        expect(rows).toHaveLength(rowCount);
        expect(rows[0].T).toBe('00:00:00');
      });

      it('should download large result set with multiple chunks from table for time', async () => {
        // Given Snowflake client is logged in
        void connection;

        // And Table with TIME column exists with 100000 sequential time values starting from 00:00:00
        const tableName = await createTemporaryTable(connection, 'COL TIME');
        await executeAsync(
          connection,
          `INSERT INTO ${tableName} (COL)
           SELECT TIMEADD(millisecond, ROW_NUMBER() OVER (ORDER BY seq4()) - 1, '00:00:00'::TIME)
           FROM TABLE(GENERATOR(ROWCOUNT => ${rowCount}))`,
        );

        // When Query "SELECT * FROM {table} ORDER BY col" is executed
        const { rows } = await executeAsync(connection, `SELECT * FROM ${tableName} ORDER BY COL`);

        // Then Result should contain 100000 sequentially increasing time values from 00:00:00
        expect(rows).toHaveLength(rowCount);
      });
    });
  });

  describe('fetchAsString', () => {
    it('should return TIME unchanged', async () => {
      const { rows } = await executeAsync(connection, `SELECT '10:30:00'::TIME`, {
        fetchAsString: ['String'],
      });
      expect(Object.values(rows[0])).toEqual(['10:30:00']);
    });

    it("should render a NULL TIME cell as the string 'NULL'", async () => {
      const { rows } = await executeAsync(connection, 'SELECT NULL::TIME', {
        fetchAsString: ['String'],
      });
      if (isRunningNewDriverWithBD('BD#14')) {
        expect(Object.values(rows[0])).toEqual([null]);
      } else {
        expect(Object.values(rows[0])).toEqual(['NULL']);
      }
    });

    it('should render a NULL TIME cell as null when representNullAsStringNull is disabled', async () => {
      const nullPreservingConnection = await createLiveNullPreservingConnection();
      const { rows } = await executeAsync(nullPreservingConnection, 'SELECT NULL::TIME', {
        fetchAsString: ['String'],
      });
      expect(Object.values(rows[0])).toEqual([null]);
    });
  });

  describe('TIME_OUTPUT_FORMAT', () => {
    // statement-level is a known bug in both drivers documented in BCR_LOG.md
    it('should ignore TIME_OUTPUT_FORMAT when set at statement-level', async () => {
      const { rows } = await executeAsync(connection, `SELECT '12:34:56.789789789'::TIME`, {
        parameters: { TIME_OUTPUT_FORMAT: 'HH24:MI:SS.FF9' },
      });
      expect(Object.values(rows[0])).toEqual(['12:34:56']);
    });

    it('should honor TIME_OUTPUT_FORMAT when set on the session', async () => {
      await setTimeOutputFormat('HH12:MI:SS AM');
      const { rows } = await executeAsync(connection, `SELECT '14:45:30'::TIME as VAL`);
      expect(rows[0].VAL).toBe('02:45:30 PM');
    });
  });
});
