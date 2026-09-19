import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import type { Connection } from '../../../types/sdk-types.js';
import { createLiveConnection, createTemporaryTable } from '../../utils/fixtures.js';
import {
  destroyConnectionAsync,
  executeAsync,
  getStatementColumn,
  NOT_IMPLEMENTED_IN_NEW_DRIVER,
} from '../../utils/index.js';
import {
  createLiveNullPreservingConnection,
  expectBigIntValue,
  expectBigIntValues,
} from '../utils.js';

describe('INT data type', () => {
  describe('tests/definitions/shared/types/int.feature', () => {
    let connection: Connection;

    beforeAll(async () => {
      // Default driver behavior is losing precision for anything higher than Number.MAX_SAFE_INTEGER
      // So we use jsTreatIntegerAsBigInt and assert every value as BigInt
      connection = await createLiveConnection({ jsTreatIntegerAsBigInt: true }, false);
    });

    afterAll(async () => {
      await destroyConnectionAsync(connection);
    });

    it.each(['INT', 'INTEGER', 'BIGINT', 'SMALLINT', 'TINYINT', 'BYTEINT'])(
      'should cast integer values to appropriate type for int and synonyms (%s)',
      async (type) => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT 0::<type>, 1000000::<type>, 9223372036854775807::<type>" is executed
        const { statement, rows } = await executeAsync(
          connection,
          `SELECT 0::${type}, 1000000::${type}, 9223372036854775807::${type}`,
        );

        // Then All values should be returned as appropriate type with no precision loss
        for (const index of [0, 1, 2]) {
          const column = getStatementColumn(statement, index);
          expect(column.getType()).toBe('fixed');
          expect(column.isNumber()).toBe(true);
          expect(column.getScale()).toBe(0);
        }
        expectBigIntValues(Object.values(rows[0]), ['0', '1000000', '9223372036854775807']);
      },
    );

    it('should select integer <values> for int and synonyms', async () => {
      // Given Snowflake client is logged in
      void connection;

      const literalCases: [query: string, expected: string[]][] = [
        ['0::INT', ['0']],
        ['-128::INT, 127::INT, 255::INT', ['-128', '127', '255']],
        ['-32768::INT, 32767::INT, 65535::INT', ['-32768', '32767', '65535']],
        [
          '-2147483648::INT, 2147483647::INT, 4294967295::INT',
          ['-2147483648', '2147483647', '4294967295'],
        ],
        [
          '-9223372036854775808::INT, 9223372036854775807::INT',
          ['-9223372036854775808', '9223372036854775807'],
        ],
      ];

      for (const [query, expected] of literalCases) {
        // When Query "SELECT <query_values>" is executed
        const { rows } = await executeAsync(connection, `SELECT ${query}`);

        // Then Result should contain integers <expected_values>
        expectBigIntValues(Object.values(rows[0]), expected);
      }
    });

    it('should handle large integer values for int and synonyms', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT -99999999999999999999999999999999999999::<type>, 99999999999999999999999999999999999999::<type>" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT
           -99999999999999999999999999999999999999::INT,
           99999999999999999999999999999999999999::INT`,
      );

      // Then Result should contain integers [-99999999999999999999999999999999999999, 99999999999999999999999999999999999999]
      expectBigIntValues(Object.values(rows[0]), [
        '-99999999999999999999999999999999999999',
        '99999999999999999999999999999999999999',
      ]);
    });

    it('should handle NULL values for int and synonyms', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT NULL::<type>, 42::<type>, NULL::<type>" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT NULL::INT, 42::INT, NULL::INT AS NULL_COL2`,
      );

      // Then Result should contain [NULL, 42, NULL]
      const [c1, c2, c3] = Object.values(rows[0]);
      expect(c1).toBeNull();
      expectBigIntValue(c2, '42');
      expect(c3).toBeNull();
    });

    it('should select <values> from table for int and synonyms', async () => {
      // Given Snowflake client is logged in
      void connection;

      const tableCases: [insert: string, expected: (string | null)[]][] = [
        [
          '(0), (1), (127), (255), (32767), (65535), (2147483647), (4294967295), (9223372036854775807)',
          [
            '0',
            '1',
            '127',
            '255',
            '32767',
            '65535',
            '2147483647',
            '4294967295',
            '9223372036854775807',
          ],
        ],
        [
          '(-1), (-128), (-32768), (-2147483648), (-9223372036854775808)',
          ['-9223372036854775808', '-2147483648', '-32768', '-128', '-1'],
        ],
        ['(0), (NULL), (42)', ['0', '42', null]],
      ];

      for (const [insert, expected] of tableCases) {
        // And Table with <type> column exists with values <insert_values>
        const tableName = await createTemporaryTable(connection, 'COL INT');
        await executeAsync(connection, `INSERT INTO ${tableName} (COL) VALUES ${insert}`);

        // When Query "SELECT * FROM <table> ORDER BY col" is executed
        const { rows } = await executeAsync(
          connection,
          `SELECT COL FROM ${tableName} ORDER BY COL NULLS LAST`,
        );

        // Then Result should contain integers <expected_values>
        expect(rows).toHaveLength(expected.length);
        expected.forEach((rowValue, index) => {
          if (rowValue === null) {
            expect(rows[index].COL).toBeNull();
          } else {
            expectBigIntValue(rows[index].COL, rowValue);
          }
        });
      }
    });

    it('should select large integer values from table for int and synonyms', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And Table with <type> column exists with values [-99999999999999999999999999999999999999, 99999999999999999999999999999999999999]
      const tableName = await createTemporaryTable(connection, 'COL INT');
      await executeAsync(
        connection,
        `INSERT INTO ${tableName} (COL) VALUES
           (-99999999999999999999999999999999999999),
           (99999999999999999999999999999999999999)`,
      );

      // When Query "SELECT * FROM <table> ORDER BY col" is executed
      const { rows } = await executeAsync(connection, `SELECT COL FROM ${tableName} ORDER BY COL`);

      // Then Result should contain integers [-99999999999999999999999999999999999999, 99999999999999999999999999999999999999]
      expectBigIntValues(
        rows.map((row) => row.COL),
        ['-99999999999999999999999999999999999999', '99999999999999999999999999999999999999'],
      );
    });

    it('should insert integer using parameter binding for int and synonyms', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And Table with <type> column exists
      const tableName = await createTemporaryTable(connection, 'COL INT');

      // When Integer values [0, -2147483648, 2147483647, 9223372036854775807] are inserted using binding
      await executeAsync(connection, `INSERT INTO ${tableName} (COL) VALUES (?)`, {
        binds: [[0], [-2147483648], [2147483647], ['9223372036854775807']],
      });

      // And Query "SELECT * FROM <table>" is executed
      const { rows } = await executeAsync(connection, `SELECT COL FROM ${tableName} ORDER BY COL`);

      // Then Result should contain integers [0, -2147483648, 2147483647, 9223372036854775807]
      expectBigIntValues(
        rows.map((row) => row.COL),
        ['-2147483648', '0', '2147483647', '9223372036854775807'],
      );
    });

    describe.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)('multiple chunks', () => {
      const ROW_COUNT = 50_000;

      it('should download large result set with multiple chunks for int and synonyms', async () => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT seq8()::<type> as id FROM TABLE(GENERATOR(ROWCOUNT => 50000)) v ORDER BY id" is executed
        const { rows } = await executeAsync(
          connection,
          `SELECT seq8()::INT AS ID FROM TABLE(GENERATOR(ROWCOUNT => ${ROW_COUNT})) v ORDER BY ID`,
        );

        // Then Result should contain 50000 sequentially numbered rows from 0 to 49999
        expect(rows).toHaveLength(ROW_COUNT);
        expectBigIntValues([rows[0].ID, rows[ROW_COUNT - 1].ID], ['0', String(ROW_COUNT - 1)]);
      });

      it('should handle server-side Arrow memory optimization for int columns on multiple chunks', async () => {
        // Given Snowflake client is logged in
        void connection;

        // And Table with four INT columns exists
        const tableName = await createTemporaryTable(
          connection,
          'COL_INT8 INT, COL_INT16 INT, COL_INT32 INT, COL_INT64 INT',
        );

        // And Each column contains values of different magnitudes (50000 rows to span multiple Arrow chunks)
        await executeAsync(
          connection,
          `INSERT INTO ${tableName} (COL_INT8, COL_INT16, COL_INT32, COL_INT64)
             SELECT
               (seq8() % 256) - 128,
               (seq8() % 65536) - 32768,
               (seq8() % 4294967296) - 2147483648,
               seq8() * 100000000000
             FROM TABLE(GENERATOR(ROWCOUNT => ${ROW_COUNT})) v`,
        );

        // When Query "SELECT * FROM <table>" is executed
        const { rows } = await executeAsync(
          connection,
          `SELECT COL_INT8, COL_INT16, COL_INT32, COL_INT64 FROM ${tableName}
             ORDER BY COL_INT64`,
        );

        // Then Result should contain 50000 rows with all values equal to expected data
        expect(rows).toHaveLength(ROW_COUNT);
        expectBigIntValues(Object.values(rows[0]), ['-128', '-32768', '-2147483648', '0']);
      });
    });
  });

  describe('fetchAsString', () => {
    it.each([true, false])(
      'should return integers beyond MAX_SAFE_INTEGER as exact digit strings without precision loss (jsTreatIntegerAsBigInt=%s)',
      async (jsTreatIntegerAsBigInt) => {
        const connection = await createLiveConnection({
          jsTreatIntegerAsBigInt,
        });

        // MAX_SAFE_INTEGER + 4 rounds to a different Number, so the string form proves precision is kept.
        const beyondSafeInteger = BigInt(Number.MAX_SAFE_INTEGER) + 4n;
        expect(String(Number(beyondSafeInteger))).not.toBe(String(beyondSafeInteger));

        const { rows } = await executeAsync(
          connection,
          `SELECT 123::INT AS C1, ${beyondSafeInteger}::INT, 1e1::INT`,
          { fetchAsString: ['Number'] },
        );
        const [small, large, exponential] = Object.values(rows[0]);
        expect([small, large, exponential]).toEqual(['123', String(beyondSafeInteger), '10']);
      },
    );

    it("should render a NULL INT cell as the string 'NULL'", async () => {
      const connection = await createLiveConnection({});
      const { rows } = await executeAsync(connection, 'SELECT NULL::INT', {
        fetchAsString: ['Number'],
      });
      expect(Object.values(rows[0])).toEqual(['NULL']);
    });

    it('should render a NULL INT cell as null when representNullAsStringNull is disabled', async () => {
      const nullPreservingConnection = await createLiveNullPreservingConnection();
      const { rows } = await executeAsync(nullPreservingConnection, 'SELECT NULL::INT', {
        fetchAsString: ['Number'],
      });
      expect(Object.values(rows[0])).toEqual([null]);
    });
  });
});
