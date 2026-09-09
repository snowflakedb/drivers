import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import type { Connection } from '../../../types/sdk-types.js';
import {
  createTestConnection,
  destroyConnectionAsync,
  executeAsync,
  getStatementColumn,
  IS_RUNNING_FOR_OLD_DRIVER,
  randomizeName,
} from '../../utils/index.js';

const INT_VEC_2D = [40, 1_234_567];
const INT_VEC_3D = [1, 2, 3];
const INT_VEC_3D_ALT = [1, 3, -5];
const INT_VEC_3D_B = [10, 20, 30];
const FLOAT_VEC_3D = [1.5, 2.5, 3.5];
const FLOAT_VEC_5D = [1.1, 2.2, 3.3, 4.4, 5.5];
const FLOAT_VEC_5D_ALT = [1.8, -3.4, 6.7, 0.0, 2.3];
const FLOAT_VEC_5D_B = [10.5, 20.5, 30.5, 40.5, 50.5];

const MAX_DIMENSION_SIZE = 4096;
const LARGE_RESULT_SET_SIZE = 20_000;
const INT32_MIN = -2_147_483_648;
const INT32_MAX = 2_147_483_647;
const FLOAT32_MAX = 3.4028235e38;
const FLOAT32_REL = 1e-6;

function expectNumberArray(actual: unknown): asserts actual is number[] {
  expect(Array.isArray(actual)).toBe(true);
  expect((actual as unknown[]).every((value) => typeof value === 'number')).toBe(true);
}

function expectFloatVector(actual: unknown, expected: number[], abs = Number.EPSILON) {
  expectNumberArray(actual);
  expect(actual).toHaveLength(expected.length);
  for (let i = 0; i < expected.length; i++) {
    if (expected[i] === 0) {
      expect(actual[i]).toBe(0);
      continue;
    }
    expect(Math.abs(actual[i] - expected[i])).toBeLessThanOrEqual(
      Math.abs(expected[i]) * FLOAT32_REL + abs,
    );
  }
}

describe.skipIf(IS_RUNNING_FOR_OLD_DRIVER)('VECTOR data type', () => {
  let connection: Connection;

  beforeAll(async () => {
    connection = createTestConnection();
    await connection.connectAsync();
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  describe('tests/definitions/shared/types/vector.feature', () => {
    it('should cast vector values to appropriate type', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT [1, 2, 3]::VECTOR(INT, 3), [1.5, 2.5, 3.5]::VECTOR(FLOAT, 3)" is executed
      const { statement, rows } = await executeAsync(
        connection,
        'SELECT [1, 2, 3]::VECTOR(INT, 3) AS C1, [1.5, 2.5, 3.5]::VECTOR(FLOAT, 3) AS C2',
      );

      // Then All values should be returned as appropriate type
      for (const index of [0, 1]) {
        expect(getStatementColumn(statement, index).getType()).toBe('vector');
      }
      const values = Object.values(rows[0]);
      expect(values[0]).toEqual(INT_VEC_3D);
      expectNumberArray(values[0]);
      expectFloatVector(values[1], FLOAT_VEC_3D);
    });

    it.each([
      {
        subtype: 'INT-3d',
        vecType: 'INT',
        sql: 'SELECT [1, 3, -5]::VECTOR(INT, 3) AS VEC',
        expected: INT_VEC_3D_ALT,
      },
      {
        subtype: 'INT-2d',
        vecType: 'INT',
        sql: 'SELECT [40, 1234567]::VECTOR(INT, 2) AS VEC',
        expected: INT_VEC_2D,
      },
      {
        subtype: 'FLOAT-5d',
        vecType: 'FLOAT',
        sql: 'SELECT [1.8, -3.4, 6.7, 0.0, 2.3]::VECTOR(FLOAT, 5) AS VEC',
        expected: FLOAT_VEC_5D_ALT,
      },
    ])('should select $subtype vector literal', async ({ vecType, sql, expected }) => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT <expected_value>::VECTOR(<vec_type>, ...)" is executed
      const { rows } = await executeAsync(connection, sql);

      // Then Result should contain <subtype> vector <expected_value>
      if (vecType === 'FLOAT') {
        expectFloatVector(rows[0].VEC, expected);
      } else {
        expect(rows[0].VEC).toEqual(expected);
        expectNumberArray(rows[0].VEC);
      }
    });

    it('should handle NULL vector values from literals', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT [1, 2, 3]::VECTOR(INT, 3), NULL::VECTOR(INT, 3), NULL::VECTOR(FLOAT, 3)" is executed
      const { rows } = await executeAsync(
        connection,
        'SELECT [1, 2, 3]::VECTOR(INT, 3) AS C1, NULL::VECTOR(INT, 3) AS C2, NULL::VECTOR(FLOAT, 3) AS C3',
      );

      // Then Result should contain [[1, 2, 3], NULL, NULL]
      expect(Object.values(rows[0])).toEqual([INT_VEC_3D, null, null]);
    });

    it.each([
      {
        subtype: 'INT',
        vecType: 'INT',
        sql: 'SELECT [-2147483648, 2147483647, 0]::VECTOR(INT, 3) AS VEC',
        expected: [INT32_MIN, INT32_MAX, 0],
      },
      {
        subtype: 'FLOAT',
        vecType: 'FLOAT',
        sql: 'SELECT [3.4028235e38, -3.4028235e38, 0.0]::VECTOR(FLOAT, 3) AS VEC',
        expected: [FLOAT32_MAX, -FLOAT32_MAX, 0.0],
      },
    ])('should select $subtype vector boundary values', async ({ vecType, sql, expected }) => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT <expected_value>::VECTOR(<vec_type>, ...)" is executed
      const { rows } = await executeAsync(connection, sql);

      // Then Result should preserve <subtype> boundary values
      if (vecType === 'FLOAT') {
        expectFloatVector(rows[0].VEC, expected, 0);
      } else {
        expect(rows[0].VEC).toEqual(expected);
        expectNumberArray(rows[0].VEC);
      }
    });

    it('should select max-dimension vector', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query selecting 4096-element float vector is executed
      const values = Array.from({ length: MAX_DIMENSION_SIZE }, (_, i) => i).join(', ');
      const { rows } = await executeAsync(
        connection,
        `SELECT [${values}]::VECTOR(FLOAT, ${MAX_DIMENSION_SIZE}) AS VEC`,
      );

      // Then Result should be a valid 4096-element float vector
      expectNumberArray(rows[0].VEC);
      expect(rows[0].VEC).toHaveLength(MAX_DIMENSION_SIZE);
      expect(rows[0].VEC[0]).toBe(0);
      expect(rows[0].VEC[MAX_DIMENSION_SIZE - 1]).toBeCloseTo(MAX_DIMENSION_SIZE - 1, 5);
    });

    it('should select vector values from table', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And Table with VECTOR(INT, 3) and VECTOR(FLOAT, 5) columns exists with values
      const tableName = randomizeName('VECTOR_TEST');
      await executeAsync(
        connection,
        `CREATE OR REPLACE TEMPORARY TABLE ${tableName} (ID INT, INT_VEC VECTOR(INT, 3), FLOAT_VEC VECTOR(FLOAT, 5))`,
      );
      try {
        await executeAsync(
          connection,
          `INSERT INTO ${tableName}
           SELECT 1, [1, 2, 3]::VECTOR(INT, 3), [1.1, 2.2, 3.3, 4.4, 5.5]::VECTOR(FLOAT, 5)
           UNION ALL SELECT 2, [10, 20, 30]::VECTOR(INT, 3), [10.5, 20.5, 30.5, 40.5, 50.5]::VECTOR(FLOAT, 5)`,
        );

        // When Query "SELECT * FROM <table> ORDER BY id" is executed
        const { rows } = await executeAsync(connection, `SELECT * FROM ${tableName} ORDER BY ID`);

        // Then Result should contain the expected integer and float vector values
        expect(rows).toHaveLength(2);
        expect(rows[0].INT_VEC).toEqual(INT_VEC_3D);
        expectNumberArray(rows[0].INT_VEC);
        expectFloatVector(rows[0].FLOAT_VEC, FLOAT_VEC_5D);
        expect(rows[1].INT_VEC).toEqual(INT_VEC_3D_B);
        expectNumberArray(rows[1].INT_VEC);
        expectFloatVector(rows[1].FLOAT_VEC, FLOAT_VEC_5D_B);
      } finally {
        await executeAsync(connection, `DROP TABLE IF EXISTS ${tableName}`);
      }
    });

    it('should handle NULL vector values from table', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And Table with VECTOR columns exist containing NULLs and values
      const tableName = randomizeName('VECTOR_NULL_TEST');
      await executeAsync(
        connection,
        `CREATE OR REPLACE TEMPORARY TABLE ${tableName} (ID INT, INT_VEC VECTOR(INT, 3), FLOAT_VEC VECTOR(FLOAT, 3))`,
      );
      try {
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
        expect(rows).toHaveLength(3);
        expect(rows[0].INT_VEC).toEqual(INT_VEC_3D);
        expect(rows[0].FLOAT_VEC).toBeNull();
        expect(rows[1].INT_VEC).toBeNull();
        expectFloatVector(rows[1].FLOAT_VEC, FLOAT_VEC_3D);
        expect(rows[2].INT_VEC).toBeNull();
        expect(rows[2].FLOAT_VEC).toBeNull();
      } finally {
        await executeAsync(connection, `DROP TABLE IF EXISTS ${tableName}`);
      }
    });

    it('should download vector data in multiple chunks', async () => {
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
        expect(rows[i].VEC).toEqual([i, i * 2, i * 3]);
        expectNumberArray(rows[i].VEC);
      }
    });
  });
});
