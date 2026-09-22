import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import type { Connection } from '../../../types/sdk-types.js';
import { createLiveConnection, createTemporaryTable } from '../../utils/fixtures.js';
import {
  destroyConnectionAsync,
  executeAsync,
  getStatementColumn,
  isRunningNewDriverWithBD,
  NOT_IMPLEMENTED_IN_NEW_DRIVER,
} from '../../utils/index.js';
import { createLiveNullPreservingConnection } from '../utils.js';

describe('FLOAT data type', () => {
  let connection: Connection;

  beforeAll(async () => {
    connection = await createLiveConnection({}, false);
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  describe('tests/definitions/shared/types/float.feature', () => {
    it.each(['FLOAT', 'FLOAT4', 'FLOAT8', 'DOUBLE', 'DOUBLE PRECISION', 'REAL'])(
      'should cast float values to appropriate type for float and synonyms (%s)',
      async (type) => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT 0.0::<type>, 123.456::<type>, 1.23e10::<type>, 'NaN'::<type>, 'inf'::<type>" is executed
        const { statement, rows } = await executeAsync(
          connection,
          `SELECT 0.0::${type}, 123.456::${type}, 1.23e10::${type}, 'NaN'::${type}, 'inf'::${type}`,
        );

        // Then All values should be returned as appropriate type
        for (const index of [0, 1, 2, 3, 4]) {
          const column = getStatementColumn(statement, index);
          expect(column.getType()).toBe('real');
          expect(column.isNumber()).toBe(true);
        }

        // And Regular values should have approximately 15 decimal digits precision
        const [zero, regular, exponential, nan, inf] = Object.values(rows[0]);
        expect([zero, regular, exponential]).toEqual([0, 123.456, 1.23e10]);

        // And NaN and inf values should be identified correctly
        expect(Number.isNaN(nan as number)).toBe(true);
        expect(inf).toBe(isRunningNewDriverWithBD('BD#9') ? Infinity : NaN);
      },
    );

    it('should select float literals for float and synonyms', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT 0.0::<type>, 1.0::<type>, -1.0::<type>, 123.456::<type>, -123.456::<type>" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT 0.0::FLOAT, 1.0::FLOAT, -1.0::FLOAT, 123.456::FLOAT, -123.456::FLOAT`,
      );

      // Then Result should contain floats [0.0, 1.0, -1.0, 123.456, -123.456]
      expect(Object.values(rows[0])).toEqual([0.0, 1.0, -1.0, 123.456, -123.456]);
    });

    it('should handle special float values from literals for float and synonyms', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT 'NaN'::<type>, 'inf'::<type>, '-inf'::<type>" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT 'NaN'::FLOAT, 'inf'::FLOAT, '-inf'::FLOAT`,
      );

      // Then Result should contain [NaN, positive_infinity, negative_infinity]
      const [nan, inf, negInf] = Object.values(rows[0]);
      expect(Number.isNaN(nan as number)).toBe(true);
      if (isRunningNewDriverWithBD('BD#9')) {
        expect([inf, negInf]).toEqual([Infinity, -Infinity]);
      } else {
        expect(Number.isNaN(inf as number)).toBe(true);
        expect(Number.isNaN(negInf as number)).toBe(true);
      }
    });

    it('should handle float <case> boundary values from literals for float and synonyms', async () => {
      // Given Snowflake client is logged in
      void connection;

      // A max-double literal overflows to Infinity and the smallest-normal literal is rounded server-side
      const queryValues = [
        '1.7976931348623157e308::FLOAT',
        '-1.7976931348623157e308::FLOAT',
        '2.2250738585072014e-308::FLOAT',
        '5e-324::FLOAT',
      ];
      const expectedValues = [Infinity, -Infinity, 2.225073859e-308, 5e-324];

      // When Query "SELECT <query_values>" is executed
      const { rows } = await executeAsync(connection, `SELECT ${queryValues.join(', ')}`);

      // Then Result should contain floats [<expected_values>]
      expect(Object.values(rows[0])).toEqual(expectedValues);
    });

    it('should handle realistic large float <case> boundary values from literals for float and synonyms', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT <query_values>" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT 1.79769313486231e+308::FLOAT, -1.79769313486231e+308::FLOAT`,
      );

      // Then Result should contain floats [<expected_values>]
      expect(Object.values(rows[0])).toEqual([1.79769313486231e308, -1.79769313486231e308]);
    });

    it('should handle float precision boundary values from literals for float and synonyms', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT 123456789012345.0::<type>, 1234567890123456.0::<type>" is executed
      const { rows } = await executeAsync(
        connection,
        // The 16-digit literal exceeds f64's ~15 significant digits, so 1234567890123456.0 rounds
        // to 1234567890123460.
        `SELECT 123456789012345.0::FLOAT, 1234567890123456.0::FLOAT`,
      );

      // Then Result should verify precision around 15 decimal digits
      expect(Object.values(rows[0])).toEqual([123456789012345.0, 1234567890123460]);
    });

    it('should handle NULL values from literals for float and synonyms', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT NULL::<type>, 42.5::<type>, NULL::<type>" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT NULL::FLOAT, 42.5::FLOAT, NULL::FLOAT AS NULL_COL2`,
      );

      // Then Result should contain [NULL, 42.5, NULL]
      expect(Object.values(rows[0])).toEqual([null, 42.5, null]);
    });

    it('should select floats from table for float and synonyms', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And Table with <type> column exists with values [0.0, 123.456, -789.012, 1.23e5, -9.87e-3]
      const tableName = await createTemporaryTable(connection, 'ID NUMBER, COL FLOAT');
      await executeAsync(
        connection,
        `INSERT INTO ${tableName} (ID, COL) VALUES
           (1, 0.0), (2, 123.456), (3, -789.012), (4, 1.23e5), (5, -9.87e-3)`,
      );

      // When Query "SELECT * FROM float_table" is executed
      const { rows } = await executeAsync(connection, `SELECT COL FROM ${tableName} ORDER BY ID`);

      // Then Result should contain floats [0.0, 123.456, -789.012, 123000.0, -0.00987]
      expect(rows.map((row) => row.COL)).toEqual([0.0, 123.456, -789.012, 123000.0, -0.00987]);
    });

    it('should handle special float values from table for float and synonyms', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And Table with <type> column exists with values [NaN, inf, -inf, 42.0, -42.0]
      const tableName = await createTemporaryTable(connection, 'ID NUMBER, COL FLOAT');
      await executeAsync(
        connection,
        `INSERT INTO ${tableName} (ID, COL) VALUES
           (1, 'NaN'::FLOAT), (2, 'inf'::FLOAT), (3, '-inf'::FLOAT), (4, 42.0), (5, -42.0)`,
      );

      // When Query "SELECT * FROM <table>" is executed
      const { rows } = await executeAsync(connection, `SELECT COL FROM ${tableName} ORDER BY ID`);

      // Then Result should contain [NaN, positive_infinity, negative_infinity, 42.0, -42.0]
      const [nan, inf, negInf, positive, negative] = rows.map((row) => row.COL);
      expect(Number.isNaN(nan as number)).toBe(true);
      expect([positive, negative]).toEqual([42.0, -42.0]);
      if (isRunningNewDriverWithBD('BD#9')) {
        expect([inf, negInf]).toEqual([Infinity, -Infinity]);
      } else {
        expect(Number.isNaN(inf as number)).toBe(true);
        expect(Number.isNaN(negInf as number)).toBe(true);
      }
    });

    it('should handle float boundary values from table for float and synonyms', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And Table with <type> column exists with boundary values [1.7976931348623157e308, -1.7976931348623157e308, 2.2250738585072014e-308, 5e-324, 123456789012345.0]
      const tableName = await createTemporaryTable(connection, 'ID NUMBER, COL FLOAT');
      await executeAsync(
        connection,
        // A max-double literal overflows to Infinity and the smallest-normal literal is rounded server-side
        `INSERT INTO ${tableName} (ID, COL) VALUES
           (1, 1.7976931348623157e308),
           (2, -1.7976931348623157e308),
           (3, 2.2250738585072014e-308),
           (4, 5e-324),
           (5, 123456789012345.0)`,
      );

      // When Query "SELECT * FROM <table>" is executed
      const { rows } = await executeAsync(connection, `SELECT COL FROM ${tableName} ORDER BY ID`);

      // Then Result should contain maximum, minimum, and precision boundary values preserved within float precision limits
      expect(rows.map((row) => row.COL)).toEqual([
        Infinity,
        -Infinity,
        2.225073859e-308,
        5e-324,
        123456789012345.0,
      ]);
    });

    it('should handle NULL values from table for float and synonyms', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And Table with <type> column exists with values [NULL, 123.456, NULL, -789.012]
      const tableName = await createTemporaryTable(connection, 'ID NUMBER, COL FLOAT');
      await executeAsync(
        connection,
        `INSERT INTO ${tableName} (ID, COL) VALUES (1, NULL), (2, 123.456), (3, NULL), (4, -789.012)`,
      );

      // When Query "SELECT * FROM <table>" is executed
      const { rows } = await executeAsync(connection, `SELECT COL FROM ${tableName} ORDER BY ID`);

      // Then Result should contain [NULL, 123.456, NULL, -789.012]
      expect(rows.map((row) => row.COL)).toEqual([null, 123.456, null, -789.012]);
    });

    it('should select large result set from table for float and synonyms', async () => {
      // Given Snowflake client is logged in
      void connection;

      const ROW_COUNT = 50_000;

      // And Table with <type> column exists with 50000 sequential values
      const tableName = await createTemporaryTable(connection, 'COL FLOAT');
      await executeAsync(
        connection,
        `INSERT INTO ${tableName} (COL)
           SELECT seq8()::FLOAT FROM TABLE(GENERATOR(ROWCOUNT => ${ROW_COUNT})) v`,
      );

      // When Query "SELECT * FROM <table>" is executed
      const { rows } = await executeAsync(connection, `SELECT COL FROM ${tableName} ORDER BY COL`);

      // Then Result should contain 50000 rows with all values returned as appropriate float type
      expect(rows).toHaveLength(ROW_COUNT);
      expect([rows[0].COL, rows[ROW_COUNT - 1].COL]).toEqual([0, ROW_COUNT - 1]);
    });

    describe('parameter binding', () => {
      it('should select float using parameter binding for float and synonyms', async () => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT ?::<type>, ?::<type>, ?::<type>" is executed with bound float values [123.456, -789.012, 42.0]
        const { rows } = await executeAsync(
          connection,
          `SELECT ?::FLOAT AS COL1, ?::FLOAT AS COL2, ?::FLOAT AS COL3`,
          { binds: [123.456, -789.012, 42.0] },
        );

        // Then Result should contain floats [123.456, -789.012, 42.0]
        expect(Object.values(rows[0])).toEqual([123.456, -789.012, 42.0]);
      });

      it('should select null float using parameter binding for float and synonyms', async () => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT ?::<type>" is executed with bound NULL value
        const { rows } = await executeAsync(connection, `SELECT ?::FLOAT`, { binds: [null] });

        // Then Result should contain NULL
        expect(Object.values(rows[0])).toEqual([null]);
      });

      it('should insert float using parameter binding for float and synonyms', async () => {
        // Given Snowflake client is logged in
        void connection;

        // And Table with <type> column exists
        const tableName = await createTemporaryTable(connection, 'ID NUMBER, COL FLOAT');

        // When Float values [0.0, 123.456, -789.012, NULL] are bulk-inserted using multirow binding
        await executeAsync(connection, `INSERT INTO ${tableName} (ID, COL) VALUES (?, ?)`, {
          binds: [
            [1, 0.0],
            [2, 123.456],
            [3, -789.012],
            [4, null],
          ],
        });

        // Then Result should contain the same values including NULL
        const { rows } = await executeAsync(connection, `SELECT COL FROM ${tableName} ORDER BY ID`);
        expect(rows.map((row) => row.COL)).toEqual([0.0, 123.456, -789.012, null]);
      });
    });

    describe.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)('multiple chunks', () => {
      const ROW_COUNT = 50_000;

      it('should download large result set with multiple chunks from GENERATOR for float and synonyms', async () => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT seq8()::<type> as id FROM TABLE(GENERATOR(ROWCOUNT => 50000)) v" is executed
        const { rows } = await executeAsync(
          connection,
          `SELECT seq8()::FLOAT AS ID FROM TABLE(GENERATOR(ROWCOUNT => ${ROW_COUNT})) v ORDER BY ID`,
        );

        // Then Result should contain 50000 rows with all values returned as appropriate float type
        expect(rows).toHaveLength(ROW_COUNT);
        expect([rows[0].ID, rows[ROW_COUNT - 1].ID]).toEqual([0, ROW_COUNT - 1]);
      });
    });
  });

  describe('fetchAsString', () => {
    it('should render FLOAT values across the spelling matrix', async () => {
      const cases = [
        { expression: '1.15::FLOAT', expectedNew: '1.15', expectedOld: '1.15' },
        { expression: '-0.5::FLOAT', expectedNew: '-0.5', expectedOld: '-0.5' },
        { expression: '1e10::FLOAT', expectedNew: '10000000000', expectedOld: '10000000000' },
        { expression: '1e-4::FLOAT', expectedNew: '0.0001', expectedOld: '0.0001' },
        { expression: '1e-5::FLOAT', expectedNew: '0.00001', expectedOld: '1e-05' },
        { expression: '1e15::FLOAT', expectedNew: '1000000000000000', expectedOld: '1e+15' },
        { expression: '-1e15::FLOAT', expectedNew: '-1000000000000000', expectedOld: '-1e+15' },
        { expression: '1e300::FLOAT', expectedNew: '1e+300', expectedOld: '1e+300' },
        { expression: '1e-300::FLOAT', expectedNew: '1e-300', expectedOld: '1e-300' },
        { expression: `'inf'::FLOAT`, expectedNew: 'inf', expectedOld: 'inf' },
        { expression: `'-inf'::FLOAT`, expectedNew: '-inf', expectedOld: '-inf' },
        { expression: `'NaN'::FLOAT`, expectedNew: 'NaN', expectedOld: 'NaN' },
      ];
      const { rows } = await executeAsync(
        connection,
        `SELECT ${cases.map(({ expression }, index) => `${expression} AS COL${index}`).join(', ')}`,
        { fetchAsString: ['Number'] },
      );
      const isNewDriver = isRunningNewDriverWithBD('BD#17');
      expect(Object.values(rows[0])).toEqual(
        cases.map(({ expectedNew, expectedOld }) => (isNewDriver ? expectedNew : expectedOld)),
      );
    });

    it("should render a NULL FLOAT cell as the string 'NULL'", async () => {
      const { rows } = await executeAsync(connection, 'SELECT NULL::FLOAT', {
        fetchAsString: ['Number'],
      });
      expect(Object.values(rows[0])).toEqual(['NULL']);
    });

    it('should render a NULL FLOAT cell as null when representNullAsStringNull is disabled', async () => {
      const nullPreservingConnection = await createLiveNullPreservingConnection();
      const { rows } = await executeAsync(nullPreservingConnection, 'SELECT NULL::FLOAT', {
        fetchAsString: ['Number'],
      });
      expect(Object.values(rows[0])).toEqual([null]);
    });
  });
});
