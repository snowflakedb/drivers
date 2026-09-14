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

describe('VECTOR data type', () => {
  let connection: Connection;

  beforeAll(async () => {
    connection = await createLiveConnection({}, false);
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  describe('tests/definitions/shared/types/vector.feature', () => {
    it('should cast vector values to appropriate type', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT [1, 2, 3]::VECTOR(INT, 3), [1.5, 2.5, 3.5]::VECTOR(FLOAT, 3)" is executed
      const { statement } = await executeAsync(
        connection,
        'SELECT [1, 2, 3]::VECTOR(INT, 3), [1.5, 2.5, 3.5]::VECTOR(FLOAT, 3)',
      );

      // Then All values should be returned as appropriate type
      for (const columnIndex of [0, 1]) {
        const column = getStatementColumn(statement, columnIndex);
        expect(column.getType()).toBe('vector');
        if (isRunningNewDriverWithBD('BD#29')) {
          expect((column as NewSnowflakeSdkColumn).isVector()).toBe(true);
        }
      }
    });

    it.each([
      {
        subtype: 'INT-3d',
        sql: 'SELECT [1, 3, -5]::VECTOR(INT, 3)',
        expected: [1, 3, -5],
        expectedOnOldDriver: '[1,3,-5]',
      },
      {
        subtype: 'INT-2d',
        sql: 'SELECT [40, 1234567]::VECTOR(INT, 2)',
        expected: [40, 1_234_567],
        expectedOnOldDriver: '[40,1234567]',
      },
      {
        subtype: 'FLOAT-5d',
        sql: 'SELECT [1.8, -3.4, 6.7, 0.0, 2.3]::VECTOR(FLOAT, 5)',
        expected: [1.8, -3.4, 6.7, 0.0, 2.3],
        expectedOnOldDriver: '[1.800000,-3.400000,6.700000,0.000000,2.300000]',
      },
    ])('should select $subtype vector literal', async ({ sql, expected, expectedOnOldDriver }) => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT <expected_value>::VECTOR(<vec_type>, ...)" is executed
      const { rows } = await executeAsync(connection, sql);

      // Then Result should contain <subtype> vector <expected_value>
      const result = Object.values(rows[0])[0];
      expect(result).toEqual(isRunningNewDriverWithBD('BD#29') ? expected : expectedOnOldDriver);
    });

    it('should handle NULL vector values from literals', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT [1, 2, 3]::VECTOR(INT, 3), NULL::VECTOR(INT, 3), NULL::VECTOR(FLOAT, 3)" is executed
      const { rows } = await executeAsync(
        connection,
        'SELECT [1, 2, 3]::VECTOR(INT, 3), NULL::VECTOR(INT, 3), NULL::VECTOR(FLOAT, 3)',
      );

      // Then Result should contain [[1, 2, 3], NULL, NULL]
      expect(Object.values(rows[0])).toEqual([
        isRunningNewDriverWithBD('BD#29') ? [1, 2, 3] : '[1,2,3]',
        null,
        null,
      ]);
    });

    it.each([
      {
        subtype: 'INT',
        sql: 'SELECT [-2147483648, 2147483647, 0]::VECTOR(INT, 3)',
        expected: [-2_147_483_648, 2_147_483_647, 0],
        expectedOnOldDriver: '[-2147483648,2147483647,0]',
      },
      {
        subtype: 'FLOAT',
        sql: 'SELECT [3.4028235e38, -3.4028235e38, 0.0]::VECTOR(FLOAT, 3)',
        expected: [3.4028235e38, -3.4028235e38, 0.0],
        expectedOnOldDriver:
          '[340282346638528859811704183484516925440.000000,-340282346638528859811704183484516925440.000000,0.000000]',
      },
    ])(
      'should select $subtype vector boundary values',
      async ({ sql, expected, expectedOnOldDriver }) => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT <expected_value>::VECTOR(<vec_type>, ...)" is executed
        const { rows } = await executeAsync(connection, sql);

        // Then Result should preserve <subtype> boundary values
        const result = Object.values(rows[0])[0];
        expect(result).toEqual(isRunningNewDriverWithBD('BD#29') ? expected : expectedOnOldDriver);
      },
    );

    // This should work once the server returns Arrow instead of JSON for the JavaScript client.
    it.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)('should preserve FLOAT smallest-normal', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query selects a VECTOR(FLOAT, ...) containing FLOAT32_SMALLEST_NORMAL
      const { rows } = await executeAsync(connection, 'SELECT [1.1754944e-38]::VECTOR(FLOAT, 1)');

      // Then the smallest-normal value must not underflow to zero
      const result = Object.values(rows[0])[0];
      expect(result).toEqual(isRunningNewDriverWithBD('BD#29') ? [1.1754944e-38] : '[0.000000]');
    });

    it('should select max-dimension vector', async () => {
      const MAX_DIMENSION_SIZE = 4096;

      // Given Snowflake client is logged in
      void connection;

      // When Query selecting 4096-element float vector is executed
      const values = Array.from({ length: MAX_DIMENSION_SIZE }, (_, i) => i);
      const { rows } = await executeAsync(
        connection,
        `SELECT [${values.join(', ')}]::VECTOR(FLOAT, ${MAX_DIMENSION_SIZE})`,
      );

      // Then Result should be a valid 4096-element float vector
      const result = Object.values(rows[0])[0];
      expect(result).toEqual(
        isRunningNewDriverWithBD('BD#29')
          ? values
          : `[${values.map((value) => value.toFixed(6)).join(',')}]`,
      );
    });

    it('should select vector values from table', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And Table with VECTOR(INT, 3) and VECTOR(FLOAT, 5) columns exists with values
      const tableName = await createTemporaryTable(
        connection,
        'ID INT, INT_VEC VECTOR(INT, 3), FLOAT_VEC VECTOR(FLOAT, 5)',
      );
      await executeAsync(
        connection,
        `INSERT INTO ${tableName}
         SELECT 1, [1, 2, 3]::VECTOR(INT, 3), [1.1, 2.2, 3.3, 4.4, 5.5]::VECTOR(FLOAT, 5)
         UNION ALL SELECT 2, [10, 20, 30]::VECTOR(INT, 3), [10.5, 20.5, 30.5, 40.5, 50.5]::VECTOR(FLOAT, 5)`,
      );

      // When Query "SELECT * FROM <table> ORDER BY id" is executed
      const { rows } = await executeAsync(connection, `SELECT * FROM ${tableName} ORDER BY ID`);

      // Then Result should contain the expected integer and float vector values
      expect(rows).toEqual(
        isRunningNewDriverWithBD('BD#29')
          ? [
              { ID: 1, INT_VEC: [1, 2, 3], FLOAT_VEC: [1.1, 2.2, 3.3, 4.4, 5.5] },
              { ID: 2, INT_VEC: [10, 20, 30], FLOAT_VEC: [10.5, 20.5, 30.5, 40.5, 50.5] },
            ]
          : [
              {
                ID: 1,
                INT_VEC: '[1,2,3]',
                FLOAT_VEC: '[1.100000,2.200000,3.300000,4.400000,5.500000]',
              },
              {
                ID: 2,
                INT_VEC: '[10,20,30]',
                FLOAT_VEC: '[10.500000,20.500000,30.500000,40.500000,50.500000]',
              },
            ],
      );
    });

    it('should handle NULL vector values from table', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And Table with VECTOR columns exist containing NULLs and values
      const tableName = await createTemporaryTable(
        connection,
        'ID INT, INT_VEC VECTOR(INT, 3), FLOAT_VEC VECTOR(FLOAT, 3)',
      );
      await executeAsync(
        connection,
        `INSERT INTO ${tableName}
         SELECT 1, [1, 2, 3]::VECTOR(INT, 3), NULL::VECTOR(FLOAT, 3)
         UNION ALL SELECT 2, NULL::VECTOR(INT, 3), [1.5, 2.5, 3.5]::VECTOR(FLOAT, 3)
         UNION ALL SELECT 3, NULL::VECTOR(INT, 3), NULL::VECTOR(FLOAT, 3)`,
      );

      // When Query "SELECT * FROM <table> ORDER BY id" is executed
      const { rows } = await executeAsync(connection, `SELECT * FROM ${tableName} ORDER BY ID`);

      // Then Result should contain both vector values and NULLs
      expect(rows).toEqual(
        isRunningNewDriverWithBD('BD#29')
          ? [
              { ID: 1, INT_VEC: [1, 2, 3], FLOAT_VEC: null },
              { ID: 2, INT_VEC: null, FLOAT_VEC: [1.5, 2.5, 3.5] },
              { ID: 3, INT_VEC: null, FLOAT_VEC: null },
            ]
          : [
              { ID: 1, INT_VEC: '[1,2,3]', FLOAT_VEC: null },
              { ID: 2, INT_VEC: null, FLOAT_VEC: '[1.500000,2.500000,3.500000]' },
              { ID: 3, INT_VEC: null, FLOAT_VEC: null },
            ],
      );
    });

    it.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)(
      'should download vector data in multiple chunks',
      async () => {
        const LARGE_RESULT_SET_SIZE = 20_000;

        // Given Snowflake client is logged in
        void connection;

        // When Query generating 20000 integer vectors is executed
        const { rows } = await executeAsync(
          connection,
          `SELECT ID, [ID, ID * 2, ID * 3]::VECTOR(INT, 3) AS VEC
         FROM (SELECT (ROW_NUMBER() OVER (ORDER BY SEQ8()) - 1) AS ID
               FROM TABLE(GENERATOR(ROWCOUNT => 20000)))
         ORDER BY ID`,
        );

        // Then All 20000 rows should be fetched with valid 3-element integer vectors
        expect(rows).toHaveLength(LARGE_RESULT_SET_SIZE);
        for (let i = 0; i < LARGE_RESULT_SET_SIZE; i++) {
          expect(rows[i].ID).toBe(i);
          expect(rows[i].VEC).toEqual(
            isRunningNewDriverWithBD('BD#29') ? [i, i * 2, i * 3] : `[${i},${i * 2},${i * 3}]`,
          );
        }
      },
    );
  });

  describe('fetchAsString', () => {
    it('should return VECTOR as string', async () => {
      const { rows } = await executeAsync(
        connection,
        'SELECT [1, 2, 3]::VECTOR(INT, 3), [1.5, 2.5, 3.5]::VECTOR(FLOAT, 3)',
        { fetchAsString: ['Number'] },
      );
      expect(Object.values(rows[0])).toEqual(
        isRunningNewDriverWithBD('BD#29')
          ? ['[1,2,3]', '[1.5,2.5,3.5]']
          : ['[1,2,3]', '[1.500000,2.500000,3.500000]'],
      );
    });

    it("should render a NULL VECTOR cell as the string 'NULL'", async () => {
      const { rows } = await executeAsync(
        connection,
        'SELECT NULL::VECTOR(INT, 3), NULL::VECTOR(FLOAT, 3)',
        {
          fetchAsString: ['Number'],
        },
      );
      expect(Object.values(rows[0])).toEqual(
        isRunningNewDriverWithBD('BD#29') ? ['NULL', 'NULL'] : [null, null],
      );
    });

    it('should render a NULL VECTOR cell as null when representNullAsStringNull is disabled', async () => {
      const nullPreservingConnection = await createLiveNullPreservingConnection();
      const { rows } = await executeAsync(
        nullPreservingConnection,
        'SELECT NULL::VECTOR(INT, 3), NULL::VECTOR(FLOAT, 3)',
        {
          fetchAsString: ['Number'],
        },
      );
      expect(Object.values(rows[0])).toEqual([null, null]);
    });
  });
});
