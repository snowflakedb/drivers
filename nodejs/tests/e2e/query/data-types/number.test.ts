import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import type { Connection } from '../../../types/sdk-types.js';
import { createLiveConnection, createTemporaryTable } from '../../utils/fixtures.js';
import {
  destroyConnectionAsync,
  executeAsync,
  getStatementColumn,
  NOT_IMPLEMENTED_IN_NEW_DRIVER,
} from '../../utils/index.js';
import { createLiveNullPreservingConnection, expectBigIntValue, isBigIntValue } from '../utils.js';

// Default driver behavior is losing precision for scaled NUMBER above Number.MAX_SAFE_INTEGER,
// and jsTreatIntegerAsBigInt only covers scale 0, so we read high-precision values through
// fetchAsString and assert string representation.
describe('NUMBER data type', () => {
  let connection: Connection;

  beforeAll(async () => {
    connection = await createLiveConnection({}, false);
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  describe('tests/definitions/shared/types/number.feature', () => {
    it.each(['NUMBER', 'DECIMAL', 'NUMERIC'])(
      'should cast number values to appropriate type for number and synonyms (%s)',
      async (type) => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT 0::<type>(10,0), 123::<type>(10,0), 0.00::<type>(10,2), 123.45::<type>(10,2)" is executed
        const { statement, rows } = await executeAsync(
          connection,
          `SELECT 0::${type}(10,0), 123::${type}(10,0), 0.00::${type}(10,2), 123.45::${type}(10,2)`,
        );

        // Then All values should be returned as appropriate type matching [0, 123, 0.00, 123.45]
        for (const index of [0, 1, 2, 3]) {
          const column = getStatementColumn(statement, index);
          expect(column.getType()).toBe('fixed');
          expect(column.isNumber()).toBe(true);
        }
        expect(getStatementColumn(statement, 0).getScale()).toBe(0);
        expect(getStatementColumn(statement, 1).getScale()).toBe(0);
        expect(getStatementColumn(statement, 2).getScale()).toBe(2);
        expect(getStatementColumn(statement, 3).getScale()).toBe(2);
        expect(Object.values(rows[0])).toEqual([0, 123, 0, 123.45]);
      },
    );

    it('should select number literals for number and synonyms', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT 0::<type>(10,0), -456::<type>(10,0), 1.50::<type>(10,2), -123.45::<type>(10,2), 123.456::<type>(15,3), -789.012::<type>(15,3)" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT 0::NUMBER(10,0), -456::NUMBER(10,0), 1.50::NUMBER(10,2), -123.45::NUMBER(10,2), 123.456::NUMBER(15,3), -789.012::NUMBER(15,3)`,
      );

      // Then Result should contain [0, -456, 1.50, -123.45, 123.456, -789.012]
      expect(Object.values(rows[0])).toEqual([0, -456, 1.5, -123.45, 123.456, -789.012]);
    });

    it('should handle high precision values from literals for number and synonyms', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT 12345678901234567890123456789012345678::<type>(38,0), 123456789012345678901234567890123456.78::<type>(38,2), 1234567890123456789012345678.1234567890::<type>(38,10), 0.0000000000000000000000000000000000001::<type>(38,37)" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT
           12345678901234567890123456789012345678::NUMBER(38,0),
           123456789012345678901234567890123456.78::NUMBER(38,2),
           1234567890123456789012345678.1234567890::NUMBER(38,10),
           0.0000000000000000000000000000000000001::NUMBER(38,37)`,
        { fetchAsString: ['Number'] },
      );

      // Then Result should contain [12345678901234567890123456789012345678, 123456789012345678901234567890123456.78, 1234567890123456789012345678.1234567890, 0.0000000000000000000000000000000000001]
      expect(Object.values(rows[0])).toEqual([
        '12345678901234567890123456789012345678',
        '123456789012345678901234567890123456.78',
        '1234567890123456789012345678.1234567890',
        '0.0000000000000000000000000000000000001',
      ]);
    });

    it('should handle scale and precision boundaries from literals for number and synonyms', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT 999.99::<type>(5,2), -999.99::<type>(5,2), 99999999::<type>(8,0), -99999999::<type>(8,0)" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT 999.99::NUMBER(5,2), -999.99::NUMBER(5,2), 99999999::NUMBER(8,0), -99999999::NUMBER(8,0)`,
      );

      // Then Result should contain [999.99, -999.99, 99999999, -99999999]
      expect(Object.values(rows[0])).toEqual([999.99, -999.99, 99999999, -99999999]);
    });

    it('should handle high precision boundaries from literals for number and synonyms', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT 99999999999999999999999999999999999999::<type>(38,0), -99999999999999999999999999999999999999::<type>(38,0)" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT
           99999999999999999999999999999999999999::NUMBER(38,0),
           -99999999999999999999999999999999999999::NUMBER(38,0)`,
        { fetchAsString: ['Number'] },
      );

      // Then Result should contain max and min 38-digit integers
      expect(Object.values(rows[0])).toEqual([
        '99999999999999999999999999999999999999',
        '-99999999999999999999999999999999999999',
      ]);
    });

    it('should handle NULL values from literals for number and synonyms', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT NULL::<type>(10,0), 42::<type>(10,0), NULL::<type>(10,2), 42.50::<type>(10,2)" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT NULL::NUMBER(10,0), 42::NUMBER(10,0), NULL::NUMBER(10,2) AS NULL_COL, 42.50::NUMBER(10,2)`,
      );

      // Then Result should contain [NULL, 42, NULL, 42.50]
      expect(Object.values(rows[0])).toEqual([null, 42, null, 42.5]);
    });

    it('should select numbers from table with multiple scales for number and synonyms', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And Table with columns (<type>(10,0), <type>(10,2), <type>(15,3), <type>(20,5)) exists
      const tableName = await createTemporaryTable(
        connection,
        'ID NUMBER, COL0 NUMBER(10,0), COL2 NUMBER(10,2), COL3 NUMBER(15,3), COL5 NUMBER(20,5)',
      );

      // And Row (123, 123.45, 123.456, 12345.67890) is inserted
      void 0;
      // And Row (-456, -67.89, -789.012, -98765.43210) is inserted
      void 0;
      // And Row (0, 0.00, 0.000, 0.00000) is inserted
      void 0;
      // And Row (999999, 999.99, 1000.500, 123456.78901) is inserted
      await executeAsync(
        connection,
        `INSERT INTO ${tableName} (ID, COL0, COL2, COL3, COL5) VALUES
           (1, 123, 123.45, 123.456, 12345.67890),
           (2, -456, -67.89, -789.012, -98765.43210),
           (3, 0, 0.00, 0.000, 0.00000),
           (4, 999999, 999.99, 1000.500, 123456.78901)`,
      );

      // When Query "SELECT * FROM <table>" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT COL0, COL2, COL3, COL5 FROM ${tableName} ORDER BY ID`,
      );

      // Then Result should contain 4 rows with expected values
      expect(rows.map((row) => Object.values(row))).toEqual([
        [123, 123.45, 123.456, 12345.6789],
        [-456, -67.89, -789.012, -98765.4321],
        [0, 0.0, 0.0, 0.0],
        [999999, 999.99, 1000.5, 123456.78901],
      ]);
    });

    it('should handle high precision values from table for number and synonyms', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And Table with columns (<type>(38,0), <type>(38,2), <type>(38,10), <type>(38,37)) exists
      const tableName = await createTemporaryTable(
        connection,
        'COL0 NUMBER(38,0), COL2 NUMBER(38,2), COL10 NUMBER(38,10), COL37 NUMBER(38,37)',
      );

      // And Row (12345678901234567890123456789012345678, 123456789012345678901234567890123456.78, 1234567890123456789012345678.1234567890, 1.2345678901234567890123456789012345678) is inserted
      await executeAsync(
        connection,
        `INSERT INTO ${tableName} (COL0, COL2, COL10, COL37) VALUES
           (12345678901234567890123456789012345678,
            123456789012345678901234567890123456.78,
            1234567890123456789012345678.1234567890,
            1.2345678901234567890123456789012345678)`,
      );

      // When Query "SELECT * FROM <table>" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT COL0, COL2, COL10, COL37 FROM ${tableName}`,
        { fetchAsString: ['Number'] },
      );

      // Then Result should contain [12345678901234567890123456789012345678, 123456789012345678901234567890123456.78, 1234567890123456789012345678.1234567890, 1.2345678901234567890123456789012345678]
      expect(Object.values(rows[0])).toEqual([
        '12345678901234567890123456789012345678',
        '123456789012345678901234567890123456.78',
        '1234567890123456789012345678.1234567890',
        '1.2345678901234567890123456789012345678',
      ]);
    });

    it('should handle scale and precision boundaries from table for number and synonyms', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And Table with columns (<type>(5,2), <type>(8,0)) exists
      const tableName = await createTemporaryTable(
        connection,
        'ID NUMBER, COL2 NUMBER(5,2), COL0 NUMBER(8,0)',
      );

      // And Row (999.99, 99999999) is inserted
      void 0;
      // And Row (-999.99, -99999999) is inserted
      void 0;
      // And Row (123.45, 12345678) is inserted
      void 0;
      // And Row (0.01, 0) is inserted
      await executeAsync(
        connection,
        `INSERT INTO ${tableName} (ID, COL2, COL0) VALUES
           (1, 999.99, 99999999),
           (2, -999.99, -99999999),
           (3, 123.45, 12345678),
           (4, 0.01, 0)`,
      );

      // When Query "SELECT * FROM <table>" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT COL2, COL0 FROM ${tableName} ORDER BY ID`,
      );

      // Then Result should contain 4 rows with expected boundary values
      expect(rows.map((row) => Object.values(row))).toEqual([
        [999.99, 99999999],
        [-999.99, -99999999],
        [123.45, 12345678],
        [0.01, 0],
      ]);
    });

    it('should handle high precision boundaries from table for number and synonyms', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And Table with columns (<type>(38,0), <type>(38,37)) exists
      const tableName = await createTemporaryTable(
        connection,
        'ID NUMBER, COL0 NUMBER(38,0), COL37 NUMBER(38,37)',
      );

      // And Row (99999999999999999999999999999999999999, 1.2345678901234567890123456789012345678) is inserted
      void 0;
      // And Row (-99999999999999999999999999999999999999, -1.2345678901234567890123456789012345678) is inserted
      void 0;
      // And Row (12345678901234567890123456789012345678, 0.0000000000000000000000000000000000001) is inserted
      await executeAsync(
        connection,
        `INSERT INTO ${tableName} (ID, COL0, COL37) VALUES
           (1, 99999999999999999999999999999999999999, 1.2345678901234567890123456789012345678),
           (2, -99999999999999999999999999999999999999, -1.2345678901234567890123456789012345678),
           (3, 12345678901234567890123456789012345678, 0.0000000000000000000000000000000000001)`,
      );

      // When Query "SELECT * FROM <table>" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT COL0, COL37 FROM ${tableName} ORDER BY ID`,
        { fetchAsString: ['Number'] },
      );

      // Then Result should contain 3 rows with expected high precision boundary values
      expect(rows.map((row) => Object.values(row))).toEqual([
        ['99999999999999999999999999999999999999', '1.2345678901234567890123456789012345678'],
        ['-99999999999999999999999999999999999999', '-1.2345678901234567890123456789012345678'],
        ['12345678901234567890123456789012345678', '0.0000000000000000000000000000000000001'],
      ]);
    });

    it('should handle NULL values from table with multiple scales for number and synonyms', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And Table with columns (<type>(10,0), <type>(10,2), <type>(15,3)) exists
      const tableName = await createTemporaryTable(
        connection,
        'ID NUMBER, COL0 NUMBER(10,0), COL2 NUMBER(10,2), COL3 NUMBER(15,3)',
      );

      // And Row (NULL, NULL, NULL) is inserted
      void 0;
      // And Row (123, 123.45, 123.456) is inserted
      void 0;
      // And Row (NULL, NULL, NULL) is inserted
      void 0;
      // And Row (-456, -67.89, -789.012) is inserted
      await executeAsync(
        connection,
        `INSERT INTO ${tableName} (ID, COL0, COL2, COL3) VALUES
           (1, NULL, NULL, NULL),
           (2, 123, 123.45, 123.456),
           (3, NULL, NULL, NULL),
           (4, -456, -67.89, -789.012)`,
      );

      // When Query "SELECT * FROM <table>" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT COL0, COL2, COL3 FROM ${tableName} ORDER BY ID`,
      );

      // Then Result should contain 4 rows with 2 NULL rows and 2 non-NULL rows with expected values
      expect(rows.map((row) => Object.values(row))).toEqual([
        [null, null, null],
        [123, 123.45, 123.456],
        [null, null, null],
        [-456, -67.89, -789.012],
      ]);
    });

    describe('parameter binding', () => {
      it('should select number using parameter binding for number and synonyms', async () => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT ?::<type>(10,0), ?::<type>(10,0), ?::<type>(10,2), ?::<type>(10,2), ?::<type>(10,0)" is executed with bound values [123, -456, 12.34, -56.78, NULL]
        const { rows } = await executeAsync(
          connection,
          `SELECT ?::NUMBER(10,0) AS COL1, ?::NUMBER(10,0) AS COL2, ?::NUMBER(10,2) AS COL3, ?::NUMBER(10,2) AS COL4, ?::NUMBER(10,0) AS COL5`,
          { binds: [123, -456, 12.34, -56.78, null] },
        );

        // Then Result should contain [123, -456, 12.34, -56.78, NULL]
        expect(Object.values(rows[0])).toEqual([123, -456, 12.34, -56.78, null]);
      });

      it('should select high precision number using parameter binding for number and synonyms', async () => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT ?::<type>(38,0), ?::<type>(38,2)" is executed with bound values [12345678901234567890123456789012345678, 123456789012345678901234567890123456.78]
        const { rows } = await executeAsync(
          connection,
          `SELECT ?::NUMBER(38,0) AS COL0, ?::NUMBER(38,2) AS COL2`,
          {
            binds: [
              '12345678901234567890123456789012345678',
              '123456789012345678901234567890123456.78',
            ],
            fetchAsString: ['Number'],
          },
        );

        // Then Result should contain [12345678901234567890123456789012345678, 123456789012345678901234567890123456.78]
        expect(Object.values(rows[0])).toEqual([
          '12345678901234567890123456789012345678',
          '123456789012345678901234567890123456.78',
        ]);
      });

      it('should insert number using parameter binding for number and synonyms', async () => {
        // Given Snowflake client is logged in
        void connection;

        // And Table with columns (<type>(10,0), <type>(10,2)) exists
        const tableName = await createTemporaryTable(
          connection,
          'ID NUMBER, COL0 NUMBER(10,0), COL2 NUMBER(10,2)',
        );

        // When Rows (0, 0.00), (123, 123.45), (-456, -67.89), (999999, 999.99), (NULL, NULL) are inserted using binding
        await executeAsync(
          connection,
          `INSERT INTO ${tableName} (ID, COL0, COL2) VALUES (?, ?, ?)`,
          {
            binds: [
              [1, 0, 0.0],
              [2, 123, 123.45],
              [3, -456, -67.89],
              [4, 999999, 999.99],
              [5, null, null],
            ],
          },
        );

        // Then Result should contain 5 rows with expected values
        const { rows } = await executeAsync(
          connection,
          `SELECT COL0, COL2 FROM ${tableName} ORDER BY ID`,
        );
        expect(rows.map((row) => Object.values(row))).toEqual([
          [0, 0.0],
          [123, 123.45],
          [-456, -67.89],
          [999999, 999.99],
          [null, null],
        ]);
      });

      it('should insert high precision number using parameter binding for number and synonyms', async () => {
        // Given Snowflake client is logged in
        void connection;

        // And Table with columns (<type>(38,0), <type>(38,2)) exists
        const tableName = await createTemporaryTable(
          connection,
          'ID NUMBER, COL0 NUMBER(38,0), COL2 NUMBER(38,2)',
        );

        // When Rows (12345678901234567890123456789012345678, 123456789012345678901234567890123456.78), (99999999999999999999999999999999999999, 0.01), (-99999999999999999999999999999999999999, -0.01) are inserted using binding
        await executeAsync(
          connection,
          `INSERT INTO ${tableName} (ID, COL0, COL2) VALUES (?, ?, ?)`,
          {
            binds: [
              [
                1,
                '12345678901234567890123456789012345678',
                '123456789012345678901234567890123456.78',
              ],
              [2, '99999999999999999999999999999999999999', '0.01'],
              [3, '-99999999999999999999999999999999999999', '-0.01'],
            ],
          },
        );

        // Then Result should contain 3 rows with expected values keeping the precision
        const { rows } = await executeAsync(
          connection,
          `SELECT COL0, COL2 FROM ${tableName} ORDER BY ID`,
          { fetchAsString: ['Number'] },
        );
        expect(rows.map((row) => Object.values(row))).toEqual([
          ['12345678901234567890123456789012345678', '123456789012345678901234567890123456.78'],
          ['99999999999999999999999999999999999999', '0.01'],
          ['-99999999999999999999999999999999999999', '-0.01'],
        ]);
      });
    });
  });

  describe.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)('multiple chunks', () => {
    const ROW_COUNT = 30_000;

    it('should download large result set with multiple chunks from GENERATOR for number and synonyms', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT seq8()::<type>(38,0), (seq8() + 0.12345)::<type>(20,5) FROM TABLE(GENERATOR(ROWCOUNT => 30000)) v" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT seq8()::NUMBER(38,0) AS COL0, (seq8() + 0.12345)::NUMBER(20,5) AS COL5
           FROM TABLE(GENERATOR(ROWCOUNT => ${ROW_COUNT})) v ORDER BY COL0`,
      );

      // Then Result should contain 30000 rows with sequential integers in column 1 and sequential decimals starting from 0.12345 in column 2
      expect(rows).toHaveLength(ROW_COUNT);
      expect(Object.values(rows[0])).toEqual([0, 0.12345]);
      expect(Object.values(rows[ROW_COUNT - 1])).toEqual([ROW_COUNT - 1, ROW_COUNT - 1 + 0.12345]);
    });

    it('should download large result set from table for number and synonyms', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And Table with columns (<type>(38,0), <type>(20,5)) exists with 30000 sequential rows, from 0 to 29999 in the first column and from 0.12345 to 29999.12345 in the second column
      const tableName = await createTemporaryTable(
        connection,
        'COL0 NUMBER(38,0), COL5 NUMBER(20,5)',
      );
      await executeAsync(
        connection,
        `INSERT INTO ${tableName} (COL0, COL5)
           SELECT seq8()::NUMBER(38,0), (seq8() + 0.12345)::NUMBER(20,5)
           FROM TABLE(GENERATOR(ROWCOUNT => ${ROW_COUNT})) v`,
      );

      // When Query "SELECT * FROM <table>" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT COL0, COL5 FROM ${tableName} ORDER BY COL0`,
      );

      // Then Result should contain 30000 rows with sequential integers in column 1 and sequential decimals starting from 0.12345 in column 2
      expect(rows).toHaveLength(ROW_COUNT);
      expect(Object.values(rows[0])).toEqual([0, 0.12345]);
      expect(Object.values(rows[ROW_COUNT - 1])).toEqual([ROW_COUNT - 1, ROW_COUNT - 1 + 0.12345]);
    });
  });

  describe('fetchAsString', () => {
    it('should render small and large NUMBER values as exact strings', async () => {
      const { rows } = await executeAsync(
        connection,
        `SELECT
           123.45::NUMBER(10,2),
           -12345678901234567890123456789012345678::NUMBER(38,0),
           123456789012345678901234567890123456.78::NUMBER(38,2)`,
        { fetchAsString: ['Number'] },
      );
      expect(Object.values(rows[0])).toEqual([
        '123.45',
        '-12345678901234567890123456789012345678',
        '123456789012345678901234567890123456.78',
      ]);
    });

    it("should render a NULL NUMBER cell as the string 'NULL'", async () => {
      const { rows } = await executeAsync(connection, 'SELECT NULL::NUMBER(10,2)', {
        fetchAsString: ['Number'],
      });
      expect(Object.values(rows[0])).toEqual(['NULL']);
    });

    it('should render a NULL NUMBER cell as null when representNullAsStringNull is disabled', async () => {
      const nullPreservingConnection = await createLiveNullPreservingConnection();
      const { rows } = await executeAsync(nullPreservingConnection, 'SELECT NULL::NUMBER(10,2)', {
        fetchAsString: ['Number'],
      });
      expect(Object.values(rows[0])).toEqual([null]);
    });
  });

  describe('jsTreatIntegerAsBigInt', () => {
    it('should return a scale-0 NUMBER as BigInt', async () => {
      const bigIntConnection = await createLiveConnection({ jsTreatIntegerAsBigInt: true });
      const { rows } = await executeAsync(
        bigIntConnection,
        `SELECT 12345678901234567890123456789012345678::NUMBER(38,0)`,
      );
      expectBigIntValue(Object.values(rows[0])[0], '12345678901234567890123456789012345678');
    });

    it('should leave a scaled NUMBER as a Number rather than a BigInt', async () => {
      const bigIntConnection = await createLiveConnection({ jsTreatIntegerAsBigInt: true });
      const { rows } = await executeAsync(
        bigIntConnection,
        `SELECT 123456789012345678901234567890.12::NUMBER(38,2)`,
      );
      expect(isBigIntValue(Object.values(rows[0])[0])).toBe(false);
    });
  });
});
