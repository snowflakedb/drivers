import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import type { Connection, NewSnowflakeSdkColumn } from '../../../types/sdk-types.js';
import { createLiveConnection, createTemporaryTable } from '../../utils/fixtures.js';
import {
  destroyConnectionAsync,
  executeAsync,
  getStatementColumn,
  isRunningNewDriverWithBD,
  NOT_IMPLEMENTED_IN_NEW_DRIVER,
} from '../../utils/index.js';
import { createLiveNullPreservingConnection } from '../utils.js';

describe('DECFLOAT data type', () => {
  let connection: Connection;

  beforeAll(async () => {
    connection = await createLiveConnection({}, false);
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  describe('tests/definitions/shared/types/decfloat.feature', () => {
    it('should cast decfloat values to appropriate type', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT 0::DECFLOAT, 123.456::DECFLOAT, 1.23e37::DECFLOAT, '12345678901234567890123456789012345678'::DECFLOAT" is executed
      const { statement, rows } = await executeAsync(
        connection,
        `SELECT 0::DECFLOAT, 123.456::DECFLOAT, 1.23e37::DECFLOAT, '12345678901234567890123456789012345678'::DECFLOAT`,
      );

      // Then All values should be returned as appropriate type
      for (const index of [0, 1, 2, 3]) {
        const column = getStatementColumn(statement, index);
        expect(column.getType()).toBe('decfloat');
        if (isRunningNewDriverWithBD('BD#6')) {
          expect((column as NewSnowflakeSdkColumn).isDecfloat()).toBe(true);
        }
      }

      // And Values should maintain full 38-digit precision
      expect(Object.values(rows[0])).toEqual([
        '0',
        '123.456',
        '12300000000000000000000000000000000000',
        '12345678901234567890123456789012345678',
      ]);
    });

    it('should select decfloat literals', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT 0::DECFLOAT, 1.5::DECFLOAT, -1.5::DECFLOAT, 123.456789::DECFLOAT, -987.654321::DECFLOAT" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT 0::DECFLOAT, 1.5::DECFLOAT, -1.5::DECFLOAT, 123.456789::DECFLOAT, -987.654321::DECFLOAT`,
      );

      // Then Result should contain exact decimals [0, 1.5, -1.5, 123.456789, -987.654321]
      expect(Object.values(rows[0])).toEqual(['0', '1.5', '-1.5', '123.456789', '-987.654321']);
    });

    it('should handle full 38-digit precision values from literals', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT '12345678901234567890123456789012345678'::DECFLOAT, '1.2345678901234567890123456789012345678E+100'::DECFLOAT, '1.2345678901234567890123456789012345678E-100'::DECFLOAT" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT '12345678901234567890123456789012345678'::DECFLOAT,
                '1.2345678901234567890123456789012345678E+100'::DECFLOAT,
                '1.2345678901234567890123456789012345678E-100'::DECFLOAT`,
      );

      // Then Result should preserve all 38 digits for each value
      expect(Object.values(rows[0])).toEqual([
        '12345678901234567890123456789012345678',
        '1.2345678901234567890123456789012345678e100',
        '1.2345678901234567890123456789012345678e-100',
      ]);
    });

    it.each([
      {
        case: 'max positive and min positive',
        queryValues: `'1E+16384'::DECFLOAT, '1E-16383'::DECFLOAT`,
        expected: ['1e16384', '1e-16383'],
      },
      {
        case: 'large negative and small positive',
        queryValues: `'-1.234E+8000'::DECFLOAT, '9.876E-8000'::DECFLOAT`,
        expected: ['-1.234e8000', '9.876e-8000'],
      },
    ])('should handle $case exponent values from literals', async ({ queryValues, expected }) => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT <query_values>" is executed
      const { rows } = await executeAsync(connection, `SELECT ${queryValues}`);

      // Then Result should contain [<expected_values>]
      expect(Object.values(rows[0])).toEqual(expected);
    });

    it('should handle NULL values from literals', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT NULL::DECFLOAT, 42.5::DECFLOAT, NULL::DECFLOAT" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT NULL::DECFLOAT AS COL1, 42.5::DECFLOAT AS COL2, NULL::DECFLOAT AS COL3`,
      );

      // Then Result should contain [NULL, 42.5, NULL]
      expect(Object.values(rows[0])).toEqual([null, '42.5', null]);
    });

    it('should select decfloats from table', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And Table with DECFLOAT column exists with values [0, 123.456, -789.012, 1.23e20, -9.87e-15]
      const tableName = await createTemporaryTable(connection, 'ID NUMBER, VAL DECFLOAT');
      await executeAsync(
        connection,
        `INSERT INTO ${tableName} (ID, VAL)
           VALUES (0, 0), (1, 123.456), (2, -789.012), (3, '1.23e20'::DECFLOAT), (4, '-9.87e-15'::DECFLOAT)`,
      );

      // When Query "SELECT * FROM <table>" is executed
      const { rows } = await executeAsync(connection, `SELECT VAL FROM ${tableName} ORDER BY ID`);

      // Then Result should contain exact decimals [0, 123.456, -789.012, 1.23e20, -9.87e-15]
      expect(rows.map((row) => row.VAL)).toEqual([
        '0',
        '123.456',
        '-789.012',
        '123000000000000000000',
        '-0.00000000000000987',
      ]);
    });

    it('should handle full 38-digit precision values from table', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And Table with DECFLOAT column exists with values [12345678901234567890123456789012345678, 1.2345678901234567890123456789012345678E+100, 1.2345678901234567890123456789012345678E-100]
      const tableName = await createTemporaryTable(connection, 'ID NUMBER, VAL DECFLOAT');
      await executeAsync(
        connection,
        `INSERT INTO ${tableName} (ID, VAL)
           VALUES (0, '12345678901234567890123456789012345678'::DECFLOAT),
                  (1, '1.2345678901234567890123456789012345678E+100'::DECFLOAT),
                  (2, '1.2345678901234567890123456789012345678E-100'::DECFLOAT)`,
      );

      // When Query "SELECT * FROM <table>" is executed
      const { rows } = await executeAsync(connection, `SELECT VAL FROM ${tableName} ORDER BY ID`);

      // Then Result should preserve all 38 digits for each value
      expect(rows.map((row) => row.VAL)).toEqual([
        '12345678901234567890123456789012345678',
        '1.2345678901234567890123456789012345678e100',
        '1.2345678901234567890123456789012345678e-100',
      ]);
    });

    it('should handle extreme exponent values from table', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And Table with DECFLOAT column exists with values [1E+16384, 1E-16383, -1.234E+8000, 9.876E-8000]
      const tableName = await createTemporaryTable(connection, 'ID NUMBER, VAL DECFLOAT');
      await executeAsync(
        connection,
        `INSERT INTO ${tableName} (ID, VAL)
           VALUES (0, '1E+16384'::DECFLOAT),
                  (1, '1E-16383'::DECFLOAT),
                  (2, '-1.234E+8000'::DECFLOAT),
                  (3, '9.876E-8000'::DECFLOAT)`,
      );

      // When Query "SELECT * FROM <table>" is executed
      const { rows } = await executeAsync(connection, `SELECT VAL FROM ${tableName} ORDER BY ID`);

      // Then Result should contain [1E+16384, 1E-16383, -1.234E+8000, 9.876E-8000]
      expect(rows.map((row) => row.VAL)).toEqual([
        '1e16384',
        '1e-16383',
        '-1.234e8000',
        '9.876e-8000',
      ]);
    });

    it('should handle NULL values from table', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And Table with DECFLOAT column exists with values [NULL, 123.456, NULL, -789.012]
      const tableName = await createTemporaryTable(connection, 'ID NUMBER, VAL DECFLOAT');
      await executeAsync(
        connection,
        `INSERT INTO ${tableName} (ID, VAL) VALUES (0, NULL), (1, 123.456), (2, NULL), (3, -789.012)`,
      );

      // When Query "SELECT * FROM <table>" is executed
      const { rows } = await executeAsync(connection, `SELECT VAL FROM ${tableName} ORDER BY ID`);

      // Then Result should contain [NULL, 123.456, NULL, -789.012]
      expect(rows.map((row) => row.VAL)).toEqual([null, '123.456', null, '-789.012']);
    });

    describe('parameter binding', () => {
      it('should select decfloat using parameter binding', async () => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT ?::DECFLOAT, ?::DECFLOAT, ?::DECFLOAT" is executed with bound DECFLOAT values [123.456, -789.012, 42.0]
        const { rows } = await executeAsync(
          connection,
          `SELECT ?::DECFLOAT AS COL1, ?::DECFLOAT AS COL2, ?::DECFLOAT AS COL3`,
          { binds: ['123.456', '-789.012', '42.0'] },
        );

        // Both old and new driver will not return trailing 0, documented in BCR_LOG.md
        // Then Result should contain [123.456, -789.012, 42.0]
        expect(Object.values(rows[0])).toEqual(['123.456', '-789.012', '42']);
      });

      it('should select null decfloat using parameter binding', async () => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT ?::DECFLOAT" is executed with bound NULL value
        const { rows } = await executeAsync(connection, `SELECT ?::DECFLOAT`, { binds: [null] });

        // Then Result should contain [NULL]
        expect(Object.values(rows[0])).toEqual([null]);
      });

      it.each([
        { case: 'max exponent', value: '1E+16384', expected: '1e16384' },
        { case: 'large negative exponent', value: '-1.234E+8000', expected: '-1.234e8000' },
      ])('should select $case decfloat using parameter binding', async ({ value, expected }) => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT ?::DECFLOAT" is executed with bound value <value>
        const { rows } = await executeAsync(connection, `SELECT ?::DECFLOAT`, { binds: [value] });

        // Then Result should contain [<expected>]
        expect(Object.values(rows[0])).toEqual([expected]);
      });

      it('should insert decfloat using parameter binding', async () => {
        // Given Snowflake client is logged in
        void connection;

        // And Table with DECFLOAT column exists
        const tableName = await createTemporaryTable(connection, 'ID NUMBER, VAL DECFLOAT');
        // When DECFLOAT values [0, 123.456, -789.012, NULL] are inserted using explicit binding
        await executeAsync(connection, `INSERT INTO ${tableName} (ID, VAL) VALUES (?, ?)`, {
          binds: [
            [0, '0'],
            [1, '123.456'],
            [2, '-789.012'],
            [3, null],
          ],
        });

        // Then SELECT should return the same exact values
        const { rows } = await executeAsync(connection, `SELECT VAL FROM ${tableName} ORDER BY ID`);
        expect(rows.map((row) => row.VAL)).toEqual(['0', '123.456', '-789.012', null]);
      });

      it('should insert extreme decfloat values using parameter binding', async () => {
        // Given Snowflake client is logged in
        void connection;

        // And Table with DECFLOAT column exists
        const tableName = await createTemporaryTable(connection, 'ID NUMBER, VAL DECFLOAT');
        // When DECFLOAT values [1E+16384, 1E-16383, -1.234E+8000] are inserted using explicit binding
        await executeAsync(connection, `INSERT INTO ${tableName} (ID, VAL) VALUES (?, ?)`, {
          binds: [
            [0, '1E+16384'],
            [1, '1E-16383'],
            [2, '-1.234E+8000'],
          ],
        });

        // And Query "SELECT * FROM <table>" is executed
        const { rows } = await executeAsync(connection, `SELECT VAL FROM ${tableName} ORDER BY ID`);

        // Then SELECT should return the same exact values
        expect(rows.map((row) => row.VAL)).toEqual(['1e16384', '1e-16383', '-1.234e8000']);
      });
    });

    describe.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)('multiple chunks', () => {
      const rowCount = 20_000;

      it('should download large result set with multiple chunks from GENERATOR', async () => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT seq8()::DECFLOAT as id FROM TABLE(GENERATOR(ROWCOUNT => 20000)) v" is executed
        const { rows } = await executeAsync(
          connection,
          `SELECT seq8()::DECFLOAT AS id FROM TABLE(GENERATOR(ROWCOUNT => ${rowCount})) v ORDER BY id`,
        );

        // Then Result should contain consecutive numbers from 0 to 19999 returned as appropriate type
        expect(rows).toHaveLength(rowCount);
        expect(rows.map((row) => row.ID)).toEqual(
          Array.from({ length: rowCount }, (_, index) => String(index)),
        );
      });

      it('should download large result set with multiple chunks from table', async () => {
        // Given Snowflake client is logged in
        void connection;

        // And Table with DECFLOAT column exists with values from 0 to 19999
        const tableName = await createTemporaryTable(connection, 'VAL DECFLOAT');
        await executeAsync(
          connection,
          `INSERT INTO ${tableName} (VAL)
             SELECT seq8()::DECFLOAT FROM TABLE(GENERATOR(ROWCOUNT => ${rowCount}))`,
        );

        // When Query "SELECT * FROM <table>" is executed
        const { rows } = await executeAsync(
          connection,
          `SELECT VAL FROM ${tableName} ORDER BY VAL`,
        );

        // Then Result should contain consecutive numbers from 0 to 19999 returned as appropriate type
        expect(rows).toHaveLength(rowCount);
        expect(rows.map((row) => row.VAL)).toEqual(
          Array.from({ length: rowCount }, (_, index) => String(index)),
        );
      });
    });
  });

  describe('fetchAsString', () => {
    it('should return DECFLOAT as string', async () => {
      const { rows } = await executeAsync(connection, `SELECT '1.5'::DECFLOAT`, {
        fetchAsString: ['String'],
      });
      expect(Object.values(rows[0])).toEqual(['1.5']);
    });

    it("should render a NULL DECFLOAT cell as the string 'NULL'", async () => {
      const { rows } = await executeAsync(connection, 'SELECT NULL::DECFLOAT', {
        fetchAsString: ['String'],
      });
      const expectedNull = isRunningNewDriverWithBD('BD#18') ? 'NULL' : null;
      expect(Object.values(rows[0])).toEqual([expectedNull]);
    });

    it('should render a NULL DECFLOAT cell as null when representNullAsStringNull is disabled', async () => {
      const nullPreservingConnection = await createLiveNullPreservingConnection();
      const { rows } = await executeAsync(nullPreservingConnection, 'SELECT NULL::DECFLOAT', {
        fetchAsString: ['String'],
      });
      expect(Object.values(rows[0])).toEqual([null]);
    });
  });

  it('should render DECFLOAT above the maximum exponent', async () => {
    const { rows } = await executeAsync(connection, `SELECT '1e16385'::DECFLOAT`);
    const value = Object.values(rows![0])[0];
    if (isRunningNewDriverWithBD('BD#7')) {
      expect(value).toBe('1e16385');
    } else {
      expect(value).toBe('10e16384');
    }
  });
});
