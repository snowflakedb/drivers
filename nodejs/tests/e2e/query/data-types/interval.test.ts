import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import type { Connection, NewSnowflakeSdkColumn } from '../../../types/sdk-types.js';
import { createLiveConnection, createTemporaryTable } from '../../utils/fixtures.js';
import {
  destroyConnectionAsync,
  executeAsync,
  isRunningNewDriverWithBD,
  NOT_IMPLEMENTED_IN_NEW_DRIVER,
  dateAtUtcMidnight,
} from '../../utils/index.js';
import { createLiveNullPreservingConnection } from '../utils.js';

// INTERVAL cells are the interval's internal integer as a string:
// - YEAR TO MONTH family: a total month count, so '1-2'::INTERVAL YEAR TO MONTH
//   is '14' (14 months).
// - DAY TO SECOND family: a total nanosecond count, so '0 0:0:1.2'::INTERVAL DAY
//   TO SECOND is '1200000000' (1.2 seconds in nanoseconds).
describe('INTERVAL data type', () => {
  let connection: Connection;

  beforeAll(async () => {
    connection = await createLiveConnection({}, false);
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  describe('tests/definitions/shared/types/interval.feature', () => {
    it('should report INTERVAL columns with dedicated type codes', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT '1-2'::INTERVAL YEAR TO MONTH, '1'::INTERVAL YEAR, '0 0:0:1.2'::INTERVAL DAY TO SECOND" is executed
      const { statement } = await executeAsync(
        connection,
        `SELECT '1-2'::INTERVAL YEAR TO MONTH,
                '1'::INTERVAL YEAR,
                '0 0:0:1.2'::INTERVAL DAY TO SECOND`,
      );

      // Then columns 0 and 1 should report type code INTERVAL_YEAR_MONTH and column 2 should report type code INTERVAL_DAY_TIME
      const columns = statement.getColumns()!;
      expect(columns[0].getType()).toBe('interval_year_month');
      expect(columns[1].getType()).toBe('interval_year_month');
      expect(columns[2].getType()).toBe('interval_day_time');
      if (isRunningNewDriverWithBD('BD#38')) {
        expect((columns[0] as NewSnowflakeSdkColumn).isIntervalYearMonth()).toBe(true);
        expect((columns[1] as NewSnowflakeSdkColumn).isIntervalYearMonth()).toBe(true);
        expect((columns[2] as NewSnowflakeSdkColumn).isIntervalDayTime()).toBe(true);
      }
    });

    it('should select INTERVAL YEAR TO MONTH literals', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query selecting INTERVAL YEAR TO MONTH literals is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT '0-0'::INTERVAL YEAR TO MONTH,
                '1-2'::INTERVAL YEAR TO MONTH,
                '-1-3'::INTERVAL YEAR TO MONTH,
                '999999999-11'::INTERVAL YEAR TO MONTH,
                '-999999999-11'::INTERVAL YEAR TO MONTH`,
      );

      // Then the result should contain expected INTERVAL YEAR TO MONTH literal values in order
      expect(Object.values(rows[0])).toEqual(['0', '14', '-15', '11999999999', '-11999999999']);
    });

    it('should select INTERVAL DAY TO SECOND literals', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query selecting INTERVAL DAY TO SECOND literals is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT '0 0:0:0.0'::INTERVAL DAY TO SECOND,
                '12 3:4:5.678'::INTERVAL DAY TO SECOND,
                '-1 2:3:4.567'::INTERVAL DAY TO SECOND,
                '99999 23:59:59.999999'::INTERVAL DAY TO SECOND,
                '-99999 23:59:59.999999'::INTERVAL DAY TO SECOND`,
      );

      // Then the result should contain expected INTERVAL DAY TO SECOND literal values in order
      expect(Object.values(rows[0])).toEqual([
        '0',
        '1047845678000000',
        '-93784567000000',
        '8639999999999999000',
        '-8639999999999999000',
      ]);
    });

    it('should select INTERVAL YEAR literals', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT '0'::INTERVAL YEAR, '1'::INTERVAL YEAR, '-1'::INTERVAL YEAR, '999999999'::INTERVAL YEAR, '-999999999'::INTERVAL YEAR" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT '0'::INTERVAL YEAR,
                '1'::INTERVAL YEAR,
                '-1'::INTERVAL YEAR,
                '999999999'::INTERVAL YEAR,
                '-999999999'::INTERVAL YEAR`,
      );

      // Then the result should contain expected INTERVAL YEAR literal values in order
      expect(Object.values(rows[0])).toEqual(['0', '12', '-12', '11999999988', '-11999999988']);
    });

    it('should select INTERVAL MONTH literals', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT '0'::INTERVAL MONTH, '1'::INTERVAL MONTH, '-1'::INTERVAL MONTH, '999999999'::INTERVAL MONTH, '-999999999'::INTERVAL MONTH" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT '0'::INTERVAL MONTH,
                '1'::INTERVAL MONTH,
                '-1'::INTERVAL MONTH,
                '999999999'::INTERVAL MONTH,
                '-999999999'::INTERVAL MONTH`,
      );

      // Then the result should contain expected INTERVAL MONTH literal values in order
      expect(Object.values(rows[0])).toEqual(['0', '1', '-1', '999999999', '-999999999']);
    });

    it('should select INTERVAL DAY literals', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT '0'::INTERVAL DAY, '1'::INTERVAL DAY, '-1'::INTERVAL DAY, '999999999'::INTERVAL DAY, '-999999999'::INTERVAL DAY" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT '0'::INTERVAL DAY,
                '1'::INTERVAL DAY,
                '-1'::INTERVAL DAY,
                '999999999'::INTERVAL DAY,
                '-999999999'::INTERVAL DAY`,
      );

      // Then the result should contain expected INTERVAL DAY literal values in order
      expect(Object.values(rows[0])).toEqual([
        '0',
        '86400000000000',
        '-86400000000000',
        '86399999913600000000000',
        '-86399999913600000000000',
      ]);
    });

    it('should select INTERVAL HOUR literals', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT '0'::INTERVAL HOUR, '1'::INTERVAL HOUR, '-1'::INTERVAL HOUR, '999999999'::INTERVAL HOUR, '-999999999'::INTERVAL HOUR" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT '0'::INTERVAL HOUR,
                '1'::INTERVAL HOUR,
                '-1'::INTERVAL HOUR,
                '999999999'::INTERVAL HOUR,
                '-999999999'::INTERVAL HOUR`,
      );

      // Then the result should contain expected INTERVAL HOUR literal values in order
      expect(Object.values(rows[0])).toEqual([
        '0',
        '3600000000000',
        '-3600000000000',
        '3599999996400000000000',
        '-3599999996400000000000',
      ]);
    });

    it('should select INTERVAL MINUTE literals', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT '0'::INTERVAL MINUTE, '1'::INTERVAL MINUTE, '-1'::INTERVAL MINUTE, '999999999'::INTERVAL MINUTE, '-999999999'::INTERVAL MINUTE" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT '0'::INTERVAL MINUTE,
                '1'::INTERVAL MINUTE,
                '-1'::INTERVAL MINUTE,
                '999999999'::INTERVAL MINUTE,
                '-999999999'::INTERVAL MINUTE`,
      );

      // Then the result should contain expected INTERVAL MINUTE literal values in order
      expect(Object.values(rows[0])).toEqual([
        '0',
        '60000000000',
        '-60000000000',
        '59999999940000000000',
        '-59999999940000000000',
      ]);
    });

    it('should select INTERVAL SECOND literals', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT '0'::INTERVAL SECOND, '1.0'::INTERVAL SECOND, '-1.0'::INTERVAL SECOND, '999999999.999999'::INTERVAL SECOND, '-999999999.999999'::INTERVAL SECOND" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT '0'::INTERVAL SECOND,
                '1.0'::INTERVAL SECOND,
                '-1.0'::INTERVAL SECOND,
                '999999999.999999'::INTERVAL SECOND,
                '-999999999.999999'::INTERVAL SECOND`,
      );

      // Then the result should contain expected INTERVAL SECOND literal values in order
      expect(Object.values(rows[0])).toEqual([
        '0',
        '1000000000',
        '-1000000000',
        '999999999999999000',
        '-999999999999999000',
      ]);
    });

    it('should select INTERVAL DAY TO HOUR literals', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT '0 0'::INTERVAL DAY TO HOUR, '1 2'::INTERVAL DAY TO HOUR, '-1 2'::INTERVAL DAY TO HOUR" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT '0 0'::INTERVAL DAY TO HOUR,
                '1 2'::INTERVAL DAY TO HOUR,
                '-1 2'::INTERVAL DAY TO HOUR`,
      );

      // Then the result should contain expected INTERVAL DAY TO HOUR literal values in order
      expect(Object.values(rows[0])).toEqual(['0', '93600000000000', '-93600000000000']);
    });

    it('should select INTERVAL DAY TO HOUR max literal', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT '999999999 23'::INTERVAL DAY TO HOUR" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT '999999999 23'::INTERVAL DAY TO HOUR`,
      );

      // Then the result should contain expected INTERVAL DAY TO HOUR max value
      expect(Object.values(rows[0])).toEqual(['86399999996400000000000']);
    });

    it('should select INTERVAL DAY TO HOUR min literal', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT '-999999999 23'::INTERVAL DAY TO HOUR" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT '-999999999 23'::INTERVAL DAY TO HOUR`,
      );

      // Then the result should contain expected INTERVAL DAY TO HOUR min value
      expect(Object.values(rows[0])).toEqual(['-86399999996400000000000']);
    });

    it('should select INTERVAL DAY TO MINUTE literals', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT '0 0:0'::INTERVAL DAY TO MINUTE, '1 2:30'::INTERVAL DAY TO MINUTE, '-1 2:30'::INTERVAL DAY TO MINUTE" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT '0 0:0'::INTERVAL DAY TO MINUTE,
                '1 2:30'::INTERVAL DAY TO MINUTE,
                '-1 2:30'::INTERVAL DAY TO MINUTE`,
      );

      // Then the result should contain expected INTERVAL DAY TO MINUTE literal values in order
      expect(Object.values(rows[0])).toEqual(['0', '95400000000000', '-95400000000000']);
    });

    it('should select INTERVAL DAY TO MINUTE max literal', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT '999999999 23:59'::INTERVAL DAY TO MINUTE" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT '999999999 23:59'::INTERVAL DAY TO MINUTE`,
      );

      // Then the result should contain expected INTERVAL DAY TO MINUTE max value
      expect(Object.values(rows[0])).toEqual(['86399999999940000000000']);
    });

    it('should select INTERVAL DAY TO MINUTE min literal', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT '-999999999 23:59'::INTERVAL DAY TO MINUTE" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT '-999999999 23:59'::INTERVAL DAY TO MINUTE`,
      );

      // Then the result should contain expected INTERVAL DAY TO MINUTE min value
      expect(Object.values(rows[0])).toEqual(['-86399999999940000000000']);
    });

    it('should select INTERVAL HOUR TO MINUTE literals', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT '0:0'::INTERVAL HOUR TO MINUTE, '1:30'::INTERVAL HOUR TO MINUTE, '-1:30'::INTERVAL HOUR TO MINUTE, '999999999:59'::INTERVAL HOUR TO MINUTE, '-999999999:59'::INTERVAL HOUR TO MINUTE" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT '0:0'::INTERVAL HOUR TO MINUTE,
                '1:30'::INTERVAL HOUR TO MINUTE,
                '-1:30'::INTERVAL HOUR TO MINUTE,
                '999999999:59'::INTERVAL HOUR TO MINUTE,
                '-999999999:59'::INTERVAL HOUR TO MINUTE`,
      );

      // Then the result should contain expected INTERVAL HOUR TO MINUTE literal values in order
      expect(Object.values(rows[0])).toEqual([
        '0',
        '5400000000000',
        '-5400000000000',
        '3599999999940000000000',
        '-3599999999940000000000',
      ]);
    });

    it('should select INTERVAL HOUR TO SECOND literals', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT '0:0:0.0'::INTERVAL HOUR TO SECOND, '1:30:45.123'::INTERVAL HOUR TO SECOND, '-1:30:45.123'::INTERVAL HOUR TO SECOND, '999999999:59:59.999999'::INTERVAL HOUR TO SECOND, '-999999999:59:59.999999'::INTERVAL HOUR TO SECOND" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT '0:0:0.0'::INTERVAL HOUR TO SECOND,
                '1:30:45.123'::INTERVAL HOUR TO SECOND,
                '-1:30:45.123'::INTERVAL HOUR TO SECOND,
                '999999999:59:59.999999'::INTERVAL HOUR TO SECOND,
                '-999999999:59:59.999999'::INTERVAL HOUR TO SECOND`,
      );

      // Then the result should contain expected INTERVAL HOUR TO SECOND literal values in order
      expect(Object.values(rows[0])).toEqual([
        '0',
        '5445123000000',
        '-5445123000000',
        '3599999999999999999000',
        '-3599999999999999999000',
      ]);
    });

    it('should select INTERVAL MINUTE TO SECOND literals', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT '0:0.0'::INTERVAL MINUTE TO SECOND, '30:45.123'::INTERVAL MINUTE TO SECOND, '-30:45.123'::INTERVAL MINUTE TO SECOND, '999999999:59.999999'::INTERVAL MINUTE TO SECOND, '-999999999:59.999999'::INTERVAL MINUTE TO SECOND" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT '0:0.0'::INTERVAL MINUTE TO SECOND,
                '30:45.123'::INTERVAL MINUTE TO SECOND,
                '-30:45.123'::INTERVAL MINUTE TO SECOND,
                '999999999:59.999999'::INTERVAL MINUTE TO SECOND,
                '-999999999:59.999999'::INTERVAL MINUTE TO SECOND`,
      );

      // Then the result should contain expected INTERVAL MINUTE TO SECOND literal values in order
      expect(Object.values(rows[0])).toEqual([
        '0',
        '1845123000000',
        '-1845123000000',
        '59999999999999999000',
        '-59999999999999999000',
      ]);
    });

    it('should select NULL INTERVAL literals', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT NULL::INTERVAL YEAR TO MONTH, NULL::INTERVAL DAY TO SECOND, NULL::INTERVAL YEAR, NULL::INTERVAL SECOND" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT NULL::INTERVAL YEAR TO MONTH,
                NULL::INTERVAL DAY TO SECOND,
                NULL::INTERVAL YEAR,
                NULL::INTERVAL SECOND`,
      );

      // Then the result should contain:
      expect(Object.values(rows[0])).toEqual([null, null, null, null]);
    });

    // TIMESTAMP_NTZ support not yet implemented to assert
    it.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)(
      'should treat INTERVAL without explicit part as seconds',
      async () => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT '2024-04-15 12:00:00'::TIMESTAMP + INTERVAL '2' AS d1, '2024-04-15 12:00:00'::TIMESTAMP + INTERVAL '2 seconds' AS d2" is executed
        const { rows } = await executeAsync(
          connection,
          `SELECT '2024-04-15 12:00:00'::TIMESTAMP + INTERVAL '2' AS d1,
                  '2024-04-15 12:00:00'::TIMESTAMP + INTERVAL '2 seconds' AS d2`,
        );

        // Then the result should contain:
        expect((rows[0].D1 as Date).toJSON()).toBe('2024-04-15 12:00:02.000');
        expect((rows[0].D2 as Date).toJSON()).toBe('2024-04-15 12:00:02.000');
      },
    );

    it('should select INTERVAL YEAR TO MONTH values from table', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And A temporary table with INTERVAL YEAR TO MONTH column is created
      const tableName = await createTemporaryTable(connection, 'C1 INTERVAL YEAR TO MONTH');

      // And The table is populated with YEAR TO MONTH values including corner cases
      await executeAsync(
        connection,
        `INSERT INTO ${tableName} (C1) VALUES
           ('-999999999-11'),
           ('-1-3'),
           ('0-0'),
           ('1-2'),
           ('999999999-11'),
           (NULL)`,
      );

      // When Query "SELECT * FROM {table} ORDER BY C1 NULLS LAST" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT * FROM ${tableName} ORDER BY C1 NULLS LAST`,
      );

      // Then the result should contain the inserted INTERVAL YEAR TO MONTH values in order
      expect(rows.map((row) => row.C1)).toEqual([
        '-11999999999',
        '-15',
        '0',
        '14',
        '11999999999',
        null,
      ]);
    });

    it('should select INTERVAL DAY TO SECOND values from table', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And A temporary table with INTERVAL DAY TO SECOND column is created
      const tableName = await createTemporaryTable(connection, 'C1 INTERVAL DAY TO SECOND');

      // And The table is populated with DAY TO SECOND values including corner cases
      await executeAsync(
        connection,
        `INSERT INTO ${tableName} (C1) VALUES
           ('0 0:0:0.0'),
           ('12 3:4:5.678'),
           ('-1 2:3:4.567'),
           ('99999 23:59:59.999999'),
           ('-99999 23:59:59.999999'),
           (NULL)`,
      );

      // When Query "SELECT * FROM {table} ORDER BY C1 NULLS LAST" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT * FROM ${tableName} ORDER BY C1 NULLS LAST`,
      );

      // Then the result should contain the inserted INTERVAL DAY TO SECOND values in order
      expect(rows.map((row) => row.C1)).toEqual([
        '-8639999999999999000',
        '-93784567000000',
        '0',
        '1047845678000000',
        '8639999999999999000',
        null,
      ]);
    });

    it('should select INTERVAL YEAR(2) TO MONTH values from table', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And A temporary table with INTERVAL YEAR(2) TO MONTH column is created
      const tableName = await createTemporaryTable(connection, 'C1 INTERVAL YEAR(2) TO MONTH');

      // And The table is populated with values ['0-0', '1-2', '-1-3', '99-11', '-99-11', NULL]
      await executeAsync(
        connection,
        `INSERT INTO ${tableName} (C1) VALUES
           ('0-0'),
           ('1-2'),
           ('-1-3'),
           ('99-11'),
           ('-99-11'),
           (NULL)`,
      );

      // When Query "SELECT * FROM {table} ORDER BY C1 NULLS LAST" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT * FROM ${tableName} ORDER BY C1 NULLS LAST`,
      );

      // Then the result should contain the inserted INTERVAL YEAR(2) TO MONTH values in order
      expect(rows.map((row) => row.C1)).toEqual(['-1199', '-15', '0', '14', '1199', null]);
    });

    it('should select INTERVAL YEAR(7) TO MONTH values from table', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And A temporary table with INTERVAL YEAR(7) TO MONTH column is created
      const tableName = await createTemporaryTable(connection, 'C1 INTERVAL YEAR(7) TO MONTH');

      // And The table is populated with values ['0-0', '1-2', '-1-3', '9999999-11', '-9999999-11', NULL]
      await executeAsync(
        connection,
        `INSERT INTO ${tableName} (C1) VALUES
           ('0-0'),
           ('1-2'),
           ('-1-3'),
           ('9999999-11'),
           ('-9999999-11'),
           (NULL)`,
      );

      // When Query "SELECT * FROM {table} ORDER BY C1 NULLS LAST" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT * FROM ${tableName} ORDER BY C1 NULLS LAST`,
      );

      // Then the result should contain the inserted INTERVAL YEAR(7) TO MONTH values in order
      expect(rows.map((row) => row.C1)).toEqual([
        '-119999999',
        '-15',
        '0',
        '14',
        '119999999',
        null,
      ]);
    });

    it('should select INTERVAL DAY(3) TO SECOND values from table', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And A temporary table with INTERVAL DAY(3) TO SECOND column is created
      const tableName = await createTemporaryTable(connection, 'C1 INTERVAL DAY(3) TO SECOND');

      // And The table is populated with values ['0 0:0:0.0', '1 2:3:4.567', '-1 2:3:4.567', '999 23:59:59.999999', '-999 23:59:59.999999', NULL]
      await executeAsync(
        connection,
        `INSERT INTO ${tableName} (C1) VALUES
           ('0 0:0:0.0'),
           ('1 2:3:4.567'),
           ('-1 2:3:4.567'),
           ('999 23:59:59.999999'),
           ('-999 23:59:59.999999'),
           (NULL)`,
      );

      // When Query "SELECT * FROM {table} ORDER BY C1 NULLS LAST" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT * FROM ${tableName} ORDER BY C1 NULLS LAST`,
      );

      // Then the result should contain the inserted INTERVAL DAY(3) TO SECOND values in order
      expect(rows.map((row) => row.C1)).toEqual([
        '-86399999999999000',
        '-93784567000000',
        '0',
        '93784567000000',
        '86399999999999000',
        null,
      ]);
    });

    it('should insert and select back INTERVAL YEAR TO MONTH values using parameter binding', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And A temporary table with INTERVAL YEAR TO MONTH column is created
      const tableName = await createTemporaryTable(connection, 'C1 INTERVAL YEAR TO MONTH');

      // When INTERVAL YEAR TO MONTH values ['0-0', '1-2', '-1-3', '999999999-11', '-999999999-11', NULL] are inserted using parameter binding
      await executeAsync(connection, `INSERT INTO ${tableName} (C1) VALUES (?)`, {
        binds: [['0-0'], ['1-2'], ['-1-3'], ['999999999-11'], ['-999999999-11'], [null]],
      });

      // And Query "SELECT * FROM {table} ORDER BY C1 NULLS LAST" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT * FROM ${tableName} ORDER BY C1 NULLS LAST`,
      );

      // Then the result should contain the bound INTERVAL YEAR TO MONTH values ['-999999999-11', '-1-3', '0-0', '1-2', '999999999-11', NULL]
      expect(rows.map((row) => row.C1)).toEqual([
        '-11999999999',
        '-15',
        '0',
        '14',
        '11999999999',
        null,
      ]);
    });

    it('should insert and select back INTERVAL DAY TO SECOND values using parameter binding', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And A temporary table with INTERVAL DAY TO SECOND column is created
      const tableName = await createTemporaryTable(connection, 'C1 INTERVAL DAY TO SECOND');

      // When INTERVAL DAY TO SECOND values ['0 0:0:0.0', '12 3:4:5.678', '-1 2:3:4.567', '99999 23:59:59.999999', '-99999 23:59:59.999999', NULL] are inserted using parameter binding
      await executeAsync(connection, `INSERT INTO ${tableName} (C1) VALUES (?)`, {
        binds: [
          ['0 0:0:0.0'],
          ['12 3:4:5.678'],
          ['-1 2:3:4.567'],
          ['99999 23:59:59.999999'],
          ['-99999 23:59:59.999999'],
          [null],
        ],
      });

      // And Query "SELECT * FROM {table} ORDER BY C1 NULLS LAST" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT * FROM ${tableName} ORDER BY C1 NULLS LAST`,
      );

      // Then the result should contain the bound INTERVAL DAY TO SECOND values ['-99999 23:59:59.999999', '-1 2:3:4.567', '0 0:0:0.0', '12 3:4:5.678', '99999 23:59:59.999999', NULL]
      expect(rows.map((row) => row.C1)).toEqual([
        '-8639999999999999000',
        '-93784567000000',
        '0',
        '1047845678000000',
        '8639999999999999000',
        null,
      ]);
    });

    it('should select INTERVAL YEAR TO MONTH values using parameter binding', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT ?::INTERVAL YEAR TO MONTH, ?::INTERVAL YEAR TO MONTH, ?::INTERVAL YEAR TO MONTH" is executed with bound string values ['0-0', '1-2', '999999999-11']
      const { rows } = await executeAsync(
        connection,
        `SELECT ?::INTERVAL YEAR TO MONTH AS COL1,
                ?::INTERVAL YEAR TO MONTH AS COL2,
                ?::INTERVAL YEAR TO MONTH AS COL3`,
        { binds: ['0-0', '1-2', '999999999-11'] },
      );

      // Then the result should contain:
      expect(Object.values(rows[0])).toEqual(['0', '14', '11999999999']);
    });

    it('should select INTERVAL DAY TO SECOND values using parameter binding', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT ?::INTERVAL DAY TO SECOND, ?::INTERVAL DAY TO SECOND, ?::INTERVAL DAY TO SECOND" is executed with bound string values ['0 0:0:0.0', '12 3:4:5.678', '99999 23:59:59.999999']
      const { rows } = await executeAsync(
        connection,
        `SELECT ?::INTERVAL DAY TO SECOND AS COL1,
                ?::INTERVAL DAY TO SECOND AS COL2,
                ?::INTERVAL DAY TO SECOND AS COL3`,
        { binds: ['0 0:0:0.0', '12 3:4:5.678', '99999 23:59:59.999999'] },
      );

      // Then the result should contain:
      expect(Object.values(rows[0])).toEqual(['0', '1047845678000000', '8639999999999999000']);
    });

    it('should select NULL INTERVAL values using parameter binding', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT ?::INTERVAL YEAR TO MONTH, ?::INTERVAL DAY TO SECOND" is executed with bound NULL values
      const { rows } = await executeAsync(
        connection,
        `SELECT ?::INTERVAL YEAR TO MONTH, ?::INTERVAL DAY TO SECOND`,
        { binds: [null, null] },
      );

      // Then the result should contain:
      expect(Object.values(rows[0])).toEqual([null, null]);
    });

    it('should select INTERVAL YEAR values using parameter binding', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT ?::INTERVAL YEAR, ?::INTERVAL YEAR, ?::INTERVAL YEAR" is executed with bound string values ['0', '2', '-999999999']
      const { rows } = await executeAsync(
        connection,
        `SELECT ?::INTERVAL YEAR AS COL1, ?::INTERVAL YEAR AS COL2, ?::INTERVAL YEAR AS COL3`,
        { binds: ['0', '2', '-999999999'] },
      );

      // Then the result should contain expected INTERVAL YEAR bound values in order
      expect(Object.values(rows[0])).toEqual(['0', '24', '-11999999988']);
    });

    it('should select INTERVAL MONTH values using parameter binding', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT ?::INTERVAL MONTH, ?::INTERVAL MONTH, ?::INTERVAL MONTH" is executed with bound string values ['0', '5', '-999999999']
      const { rows } = await executeAsync(
        connection,
        `SELECT ?::INTERVAL MONTH AS COL1, ?::INTERVAL MONTH AS COL2, ?::INTERVAL MONTH AS COL3`,
        { binds: ['0', '5', '-999999999'] },
      );

      // Then the result should contain expected INTERVAL MONTH bound values in order
      expect(Object.values(rows[0])).toEqual(['0', '5', '-999999999']);
    });

    it('should select INTERVAL DAY values using parameter binding', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT ?::INTERVAL DAY, ?::INTERVAL DAY, ?::INTERVAL DAY" is executed with bound string values ['0', '1', '-999999999']
      const { rows } = await executeAsync(
        connection,
        `SELECT ?::INTERVAL DAY AS COL1, ?::INTERVAL DAY AS COL2, ?::INTERVAL DAY AS COL3`,
        { binds: ['0', '1', '-999999999'] },
      );

      // Then the result should contain expected INTERVAL DAY bound values in order
      expect(Object.values(rows[0])).toEqual(['0', '86400000000000', '-86399999913600000000000']);
    });

    it('should select INTERVAL HOUR values using parameter binding', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT ?::INTERVAL HOUR, ?::INTERVAL HOUR, ?::INTERVAL HOUR" is executed with bound string values ['0', '5', '-999999999']
      const { rows } = await executeAsync(
        connection,
        `SELECT ?::INTERVAL HOUR AS COL1, ?::INTERVAL HOUR AS COL2, ?::INTERVAL HOUR AS COL3`,
        { binds: ['0', '5', '-999999999'] },
      );

      // Then the result should contain expected INTERVAL HOUR bound values in order
      expect(Object.values(rows[0])).toEqual(['0', '18000000000000', '-3599999996400000000000']);
    });

    it('should select INTERVAL MINUTE values using parameter binding', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT ?::INTERVAL MINUTE, ?::INTERVAL MINUTE, ?::INTERVAL MINUTE" is executed with bound string values ['0', '4', '-999999999']
      const { rows } = await executeAsync(
        connection,
        `SELECT ?::INTERVAL MINUTE AS COL1, ?::INTERVAL MINUTE AS COL2, ?::INTERVAL MINUTE AS COL3`,
        { binds: ['0', '4', '-999999999'] },
      );

      // Then the result should contain expected INTERVAL MINUTE bound values in order
      expect(Object.values(rows[0])).toEqual(['0', '240000000000', '-59999999940000000000']);
    });

    it('should select INTERVAL SECOND values using parameter binding', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT ?::INTERVAL SECOND, ?::INTERVAL SECOND, ?::INTERVAL SECOND" is executed with bound string values ['0', '8.5', '-999999999.999999']
      const { rows } = await executeAsync(
        connection,
        `SELECT ?::INTERVAL SECOND AS COL1, ?::INTERVAL SECOND AS COL2, ?::INTERVAL SECOND AS COL3`,
        { binds: ['0', '8.5', '-999999999.999999'] },
      );

      // Then the result should contain expected INTERVAL SECOND bound values in order
      expect(Object.values(rows[0])).toEqual(['0', '8500000000', '-999999999999999000']);
    });

    it('should select INTERVAL DAY TO HOUR value using parameter binding', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT ?::INTERVAL DAY TO HOUR" is executed with bound string value '1 2'
      const { rows } = await executeAsync(connection, `SELECT ?::INTERVAL DAY TO HOUR`, {
        binds: ['1 2'],
      });

      // Then the result should contain expected INTERVAL DAY TO HOUR bound value
      expect(Object.values(rows[0])).toEqual(['93600000000000']);
    });

    it('should select INTERVAL DAY TO HOUR max value using parameter binding', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT ?::INTERVAL DAY TO HOUR" is executed with bound string value '999999999 23'
      const { rows } = await executeAsync(connection, `SELECT ?::INTERVAL DAY TO HOUR`, {
        binds: ['999999999 23'],
      });

      // Then the result should contain expected INTERVAL DAY TO HOUR max bound value
      expect(Object.values(rows[0])).toEqual(['86399999996400000000000']);
    });

    it('should select INTERVAL DAY TO HOUR min value using parameter binding', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT ?::INTERVAL DAY TO HOUR" is executed with bound string value '-999999999 23'
      const { rows } = await executeAsync(connection, `SELECT ?::INTERVAL DAY TO HOUR`, {
        binds: ['-999999999 23'],
      });

      // Then the result should contain expected INTERVAL DAY TO HOUR min bound value
      expect(Object.values(rows[0])).toEqual(['-86399999996400000000000']);
    });

    it('should select INTERVAL DAY TO MINUTE value using parameter binding', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT ?::INTERVAL DAY TO MINUTE" is executed with bound string value '1 2:30'
      const { rows } = await executeAsync(connection, `SELECT ?::INTERVAL DAY TO MINUTE`, {
        binds: ['1 2:30'],
      });

      // Then the result should contain expected INTERVAL DAY TO MINUTE bound value
      expect(Object.values(rows[0])).toEqual(['95400000000000']);
    });

    it('should select INTERVAL DAY TO MINUTE max value using parameter binding', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT ?::INTERVAL DAY TO MINUTE" is executed with bound string value '999999999 23:59'
      const { rows } = await executeAsync(connection, `SELECT ?::INTERVAL DAY TO MINUTE`, {
        binds: ['999999999 23:59'],
      });

      // Then the result should contain expected INTERVAL DAY TO MINUTE max bound value
      expect(Object.values(rows[0])).toEqual(['86399999999940000000000']);
    });

    it('should select INTERVAL DAY TO MINUTE min value using parameter binding', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT ?::INTERVAL DAY TO MINUTE" is executed with bound string value '-999999999 23:59'
      const { rows } = await executeAsync(connection, `SELECT ?::INTERVAL DAY TO MINUTE`, {
        binds: ['-999999999 23:59'],
      });

      // Then the result should contain expected INTERVAL DAY TO MINUTE min bound value
      expect(Object.values(rows[0])).toEqual(['-86399999999940000000000']);
    });

    it('should select INTERVAL HOUR TO MINUTE values using parameter binding', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT ?::INTERVAL HOUR TO MINUTE, ?::INTERVAL HOUR TO MINUTE" is executed with bound string values ['1:30', '-999999999:59']
      const { rows } = await executeAsync(
        connection,
        `SELECT ?::INTERVAL HOUR TO MINUTE AS COL1, ?::INTERVAL HOUR TO MINUTE AS COL2`,
        { binds: ['1:30', '-999999999:59'] },
      );

      // Then the result should contain expected INTERVAL HOUR TO MINUTE bound values in order
      expect(Object.values(rows[0])).toEqual(['5400000000000', '-3599999999940000000000']);
    });

    it('should select INTERVAL HOUR TO SECOND values using parameter binding', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT ?::INTERVAL HOUR TO SECOND, ?::INTERVAL HOUR TO SECOND" is executed with bound string values ['1:30:45.123', '-999999999:59:59.999999']
      const { rows } = await executeAsync(
        connection,
        `SELECT ?::INTERVAL HOUR TO SECOND AS COL1, ?::INTERVAL HOUR TO SECOND AS COL2`,
        { binds: ['1:30:45.123', '-999999999:59:59.999999'] },
      );

      // Then the result should contain expected INTERVAL HOUR TO SECOND bound values in order
      expect(Object.values(rows[0])).toEqual(['5445123000000', '-3599999999999999999000']);
    });

    it('should select INTERVAL MINUTE TO SECOND values using parameter binding', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT ?::INTERVAL MINUTE TO SECOND, ?::INTERVAL MINUTE TO SECOND" is executed with bound string values ['30:45.123', '-999999999:59.999999']
      const { rows } = await executeAsync(
        connection,
        `SELECT ?::INTERVAL MINUTE TO SECOND AS COL1, ?::INTERVAL MINUTE TO SECOND AS COL2`,
        { binds: ['30:45.123', '-999999999:59.999999'] },
      );

      // Then the result should contain expected INTERVAL MINUTE TO SECOND bound values in order
      expect(Object.values(rows[0])).toEqual(['1845123000000', '-59999999999999999000']);
    });

    describe.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)('multiple chunks', () => {
      const CHUNK_ROW_COUNT = 50_000;

      it('should download INTERVAL YEAR TO MONTH data in multiple chunks', async () => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT '0-1'::INTERVAL YEAR TO MONTH * SEQ4() AS ym FROM TABLE(GENERATOR(ROWCOUNT => 50000)) v ORDER BY ym" is executed
        const { rows } = await executeAsync(
          connection,
          `SELECT '0-1'::INTERVAL YEAR TO MONTH * SEQ4() AS ym
             FROM TABLE(GENERATOR(ROWCOUNT => ${CHUNK_ROW_COUNT})) v
             ORDER BY ym`,
        );

        // Then there are 50000 rows returned
        expect(rows).toHaveLength(CHUNK_ROW_COUNT);

        // And all returned INTERVAL YEAR TO MONTH values should form a sequential series of months starting at 0
        expect([rows[0].YM, rows[12].YM, rows[25_000].YM, rows[49_999].YM]).toEqual([
          '0',
          '12',
          '25000',
          '49999',
        ]);
      });

      it('should download INTERVAL DAY TO SECOND data in multiple chunks', async () => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT '0 0:0:1.0'::INTERVAL DAY TO SECOND * SEQ4() AS dt FROM TABLE(GENERATOR(ROWCOUNT => 50000)) v ORDER BY dt" is executed
        const { rows } = await executeAsync(
          connection,
          `SELECT '0 0:0:1.0'::INTERVAL DAY TO SECOND * SEQ4() AS dt
             FROM TABLE(GENERATOR(ROWCOUNT => ${CHUNK_ROW_COUNT})) v
             ORDER BY dt`,
        );

        // Then there are 50000 rows returned
        expect(rows).toHaveLength(CHUNK_ROW_COUNT);

        // And all returned INTERVAL DAY TO SECOND values should form a sequential series of seconds starting at 0
        expect([rows[0].DT, rows[3_600].DT, rows[25_000].DT, rows[49_999].DT]).toEqual([
          '0',
          '3600000000000',
          '25000000000000',
          '49999000000000',
        ]);
      });
    });

    it('should respect order of interval components in date arithmetic', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT TO_DATE('2019-02-28') + INTERVAL '1 day, 1 year' AS d1, TO_DATE('2019-02-28') + INTERVAL '1 year, 1 day' AS d2" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT TO_DATE('2019-02-28') + INTERVAL '1 day, 1 year' AS d1,
                TO_DATE('2019-02-28') + INTERVAL '1 year, 1 day' AS d2`,
      );

      // Then the result should contain:
      expect(Object.values(rows[0])).toEqual([
        dateAtUtcMidnight('2020-03-01'),
        dateAtUtcMidnight('2020-02-29'),
      ]);
    });

    // TIMESTAMP_NTZ support not yet implemented to assert
    it.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)(
      'should support complex INTERVAL with mixed units and abbreviations',
      async () => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT TO_DATE('2025-01-17') + INTERVAL '1 y, 3 q, 4 mm, 5 w, 6 d, 7 h, 9 m, 8 s, 1000 ms, 445343232 us, 898498273498 ns' AS complex_interval" is executed
        const { rows } = await executeAsync(
          connection,
          `SELECT TO_DATE('2025-01-17') + INTERVAL '1 y, 3 q, 4 mm, 5 w, 6 d, 7 h, 9 m, 8 s, 1000 ms, 445343232 us, 898498273498 ns' AS complex_interval`,
        );

        // Then the result should contain:
        expect((rows[0].COMPLEX_INTERVAL as Date).toJSON()).toBe('2027-03-30 07:31:32.841');
      },
    );

    it('should add two INTERVAL YEAR TO MONTH values', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT '1-2'::INTERVAL YEAR TO MONTH + '0-3'::INTERVAL YEAR TO MONTH AS i" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT '1-2'::INTERVAL YEAR TO MONTH + '0-3'::INTERVAL YEAR TO MONTH AS i`,
      );

      // Then the result should contain expected INTERVAL YEAR TO MONTH value '1-5'
      expect(Object.values(rows[0])).toEqual(['17']);
    });

    it('should add two INTERVAL DAY TO SECOND values', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT '1 2:30:00.0'::INTERVAL DAY TO SECOND + '0 1:45:30.5'::INTERVAL DAY TO SECOND AS i" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT '1 2:30:00.0'::INTERVAL DAY TO SECOND + '0 1:45:30.5'::INTERVAL DAY TO SECOND AS i`,
      );

      // Then the result should contain expected INTERVAL DAY TO SECOND value '1 4:15:30.500000'
      expect(Object.values(rows[0])).toEqual(['101730500000000']);
    });

    it('should negate an INTERVAL value', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT -('1-6'::INTERVAL YEAR TO MONTH) AS ym, -('3 12:0:0.0'::INTERVAL DAY TO SECOND) AS dt" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT -('1-6'::INTERVAL YEAR TO MONTH) AS ym, -('3 12:0:0.0'::INTERVAL DAY TO SECOND) AS dt`,
      );

      // Then the result should contain expected negated INTERVAL values '-1-6' and '-3 12:0:0.000000'
      expect(Object.values(rows[0])).toEqual(['-18', '-302400000000000']);
    });

    it('should subtract two INTERVAL values', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT '1-5'::INTERVAL YEAR TO MONTH - '0-3'::INTERVAL YEAR TO MONTH AS ym, '1 4:15:30.5'::INTERVAL DAY TO SECOND - '0 1:45:30.5'::INTERVAL DAY TO SECOND AS dt" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT '1-5'::INTERVAL YEAR TO MONTH - '0-3'::INTERVAL YEAR TO MONTH AS ym,
                '1 4:15:30.5'::INTERVAL DAY TO SECOND - '0 1:45:30.5'::INTERVAL DAY TO SECOND AS dt`,
      );

      // Then the result should contain expected INTERVAL values '1-2' and '1 2:30:00.000000'
      expect(Object.values(rows[0])).toEqual(['14', '95400000000000']);
    });

    it('should multiply INTERVAL by a scalar', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT '0-6'::INTERVAL YEAR TO MONTH * 3 AS ym, 2 * '1 0:0:0.0'::INTERVAL DAY TO SECOND AS dt" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT '0-6'::INTERVAL YEAR TO MONTH * 3 AS ym, 2 * '1 0:0:0.0'::INTERVAL DAY TO SECOND AS dt`,
      );

      // Then the result should contain expected INTERVAL values '1-6' and '2 0:0:0.000000'
      expect(Object.values(rows[0])).toEqual(['18', '172800000000000']);
    });

    it('should divide INTERVAL by a scalar', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT '1-6'::INTERVAL YEAR TO MONTH / 3 AS ym, '2 0:0:0.0'::INTERVAL DAY TO SECOND / 2 AS dt" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT '1-6'::INTERVAL YEAR TO MONTH / 3 AS ym, '2 0:0:0.0'::INTERVAL DAY TO SECOND / 2 AS dt`,
      );

      // Then the result should contain expected INTERVAL values '0-6' and '1 0:0:0.000000'
      expect(Object.values(rows[0])).toEqual(['6', '86400000000000']);
    });
  });

  describe('fetchAsString', () => {
    it('should return INTERVAL YEAR TO MONTH and INTERVAL DAY TO SECOND as string', async () => {
      const { rows } = await executeAsync(
        connection,
        `SELECT '1-2'::INTERVAL YEAR TO MONTH,
                '0 0:0:1.2'::INTERVAL DAY TO SECOND`,
        { fetchAsString: ['String'] },
      );
      expect(Object.values(rows[0])).toEqual(['14', '1200000000']);
    });

    it("should render NULL INTERVAL YEAR TO MONTH and INTERVAL DAY TO SECOND cells as the string 'NULL'", async () => {
      const { rows } = await executeAsync(
        connection,
        `SELECT NULL::INTERVAL YEAR TO MONTH, NULL::INTERVAL DAY TO SECOND`,
        { fetchAsString: ['String'] },
      );
      const expectedNull = isRunningNewDriverWithBD('BD#18') ? 'NULL' : null;
      expect(Object.values(rows[0])).toEqual([expectedNull, expectedNull]);
    });

    it('should render NULL INTERVAL YEAR TO MONTH and INTERVAL DAY TO SECOND cells as null when representNullAsStringNull is disabled', async () => {
      const nullPreservingConnection = await createLiveNullPreservingConnection();
      const { rows } = await executeAsync(
        nullPreservingConnection,
        `SELECT NULL::INTERVAL YEAR TO MONTH, NULL::INTERVAL DAY TO SECOND`,
        { fetchAsString: ['String'] },
      );
      expect(Object.values(rows[0])).toEqual([null, null]);
    });
  });
});
