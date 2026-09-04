import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import type { Connection } from '../../../types/sdk-types.js';
import {
  createTestConnection,
  destroyConnectionAsync,
  executeAsync,
  getStatementColumn,
  getSnowflakeSDK,
  NOT_IMPLEMENTED_IN_NEW_DRIVER,
  randomizeName,
} from '../../utils/index.js';
import { withNullPreservingConnection } from '../utils.js';

describe('BOOLEAN data type', () => {
  const snowflake = getSnowflakeSDK();
  let connection: Connection;

  beforeAll(async () => {
    connection = createTestConnection(snowflake);
    await connection.connectAsync();
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  describe('tests/definitions/shared/types/boolean.feature', () => {
    it('should cast boolean values to appropriate type', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN, TRUE::BOOLEAN" is executed
      const { statement, rows } = await executeAsync(
        connection,
        `SELECT TRUE::BOOLEAN AS C1, FALSE::BOOLEAN AS C2, TRUE::BOOLEAN AS C3`,
      );

      // Then All values should be returned as appropriate type
      for (const index of [0, 1, 2]) {
        const column = getStatementColumn(statement, index);
        expect(column.getType()).toBe('boolean');
        expect(column.isBoolean()).toBe(true);
      }

      // And Values should match [TRUE, FALSE, TRUE]
      expect(Object.values(rows[0])).toEqual([true, false, true]);
    });

    it('should select boolean literals', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN" is executed
      const { rows } = await executeAsync(connection, `SELECT TRUE::BOOLEAN, FALSE::BOOLEAN`);

      // Then Result should contain [TRUE, FALSE]
      expect(Object.values(rows[0])).toEqual([true, false]);
    });

    it('should handle NULL values from literals', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT FALSE::BOOLEAN, NULL::BOOLEAN, TRUE::BOOLEAN, NULL::BOOLEAN" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT FALSE::BOOLEAN AS C1, NULL::BOOLEAN AS C2, TRUE::BOOLEAN AS C3, NULL::BOOLEAN AS C4`,
      );

      // Then Result should contain [FALSE, NULL, TRUE, NULL]
      expect(Object.values(rows[0])).toEqual([false, null, true, null]);
    });

    it('should select boolean values from table', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And Table with columns (BOOLEAN, BOOLEAN, BOOLEAN) exists
      const tableName = randomizeName('BOOLEAN_TEST');
      await executeAsync(
        connection,
        `CREATE OR REPLACE TEMPORARY TABLE ${tableName} (C1 BOOLEAN, C2 BOOLEAN, C3 BOOLEAN)`,
      );
      try {
        // And Row (TRUE, FALSE, TRUE) is inserted
        await executeAsync(connection, `INSERT INTO ${tableName} VALUES (TRUE, FALSE, TRUE)`);

        // When Query "SELECT * FROM <table>" is executed
        const { rows } = await executeAsync(connection, `SELECT * FROM ${tableName}`);

        // Then Result should contain [TRUE, FALSE, TRUE]
        expect(Object.values(rows[0])).toEqual([true, false, true]);
      } finally {
        await executeAsync(connection, `DROP TABLE IF EXISTS ${tableName}`);
      }
    });

    it('should handle NULL values from table', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And Table with BOOLEAN column exists
      const tableName = randomizeName('BOOLEAN_NULL_TEST');
      await executeAsync(
        connection,
        `CREATE OR REPLACE TEMPORARY TABLE ${tableName} (ID NUMBER, VAL BOOLEAN)`,
      );
      try {
        // And Rows [NULL, TRUE, FALSE] are inserted
        await executeAsync(
          connection,
          `INSERT INTO ${tableName} (ID, VAL) VALUES (1, NULL), (2, TRUE), (3, FALSE)`,
        );

        // When Query "SELECT * FROM <table>" is executed
        const { rows } = await executeAsync(connection, `SELECT VAL FROM ${tableName} ORDER BY ID`);

        // Then Result should contain [NULL, TRUE, FALSE] in any order
        expect(rows.map((row) => row.VAL)).toEqual([null, true, false]);
      } finally {
        await executeAsync(connection, `DROP TABLE IF EXISTS ${tableName}`);
      }
    });

    describe.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)('parameter binding', () => {
      it('should select boolean using parameter binding', async () => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT ?::BOOLEAN, ?::BOOLEAN, ?::BOOLEAN" is executed with bound boolean values [TRUE, FALSE, TRUE]
        const { rows } = await executeAsync(
          connection,
          'SELECT ?::BOOLEAN AS C1, ?::BOOLEAN AS C2, ?::BOOLEAN AS C3',
          { binds: [true, false, true] },
        );

        // Then Result should contain [TRUE, FALSE, TRUE]
        expect(Object.values(rows[0])).toEqual([true, false, true]);
      });

      it('should select null boolean using parameter binding', async () => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT ?::BOOLEAN" is executed with bound NULL value
        const { rows } = await executeAsync(connection, 'SELECT ?::BOOLEAN', { binds: [null] });

        // Then Result should contain [NULL]
        expect(Object.values(rows[0])).toEqual([null]);
      });

      it('should insert boolean using parameter binding', async () => {
        // Given Snowflake client is logged in
        void connection;

        // And Table with BOOLEAN column exists
        const tableName = randomizeName('BOOLEAN_BIND_TEST');
        await executeAsync(
          connection,
          `CREATE OR REPLACE TEMPORARY TABLE ${tableName} (ID NUMBER, VAL BOOLEAN)`,
        );
        try {
          // When Boolean values [TRUE, FALSE, NULL] are bulk-inserted using multirow binding
          await executeAsync(connection, `INSERT INTO ${tableName} (ID, VAL) VALUES (?, ?)`, {
            binds: [
              [1, true],
              [2, false],
              [3, null],
            ],
          });

          // Then SELECT should return the same values in any order
          const { rows } = await executeAsync(
            connection,
            `SELECT VAL FROM ${tableName} ORDER BY ID`,
          );
          expect(rows.map((row) => row.VAL)).toEqual([true, false, null]);
        } finally {
          await executeAsync(connection, `DROP TABLE IF EXISTS ${tableName}`);
        }
      });
    });

    describe.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)('multiple chunks', () => {
      const HALF = 500_000;

      it('should download large result set with multiple chunks from GENERATOR', async () => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT (id % 2 = 0)::BOOLEAN FROM <generator>" is executed
        const { rows } = await executeAsync(
          connection,
          `SELECT (seq8() % 2 = 0)::BOOLEAN AS VAL FROM TABLE(GENERATOR(ROWCOUNT => ${2 * HALF})) v`,
        );

        // Then Result should contain 500000 TRUE and 500000 FALSE values
        const trueCount = rows.filter((row) => row.VAL === true).length;
        expect(trueCount).toBe(HALF);
        expect(rows.length - trueCount).toBe(HALF);
      });

      it('should download large result set with multiple chunks from table', async () => {
        // Given Snowflake client is logged in
        void connection;

        // And Table with BOOLEAN column exists with 500000 TRUE and 500000 FALSE values
        const tableName = randomizeName('BOOLEAN_CHUNK_TEST');
        await executeAsync(
          connection,
          `CREATE OR REPLACE TEMPORARY TABLE ${tableName} (COL BOOLEAN)`,
        );
        try {
          await executeAsync(
            connection,
            `INSERT INTO ${tableName}
             SELECT (seq8() % 2 = 0)::BOOLEAN FROM TABLE(GENERATOR(ROWCOUNT => ${2 * HALF})) v`,
          );

          // When Query "SELECT col FROM <table>" is executed
          const { rows } = await executeAsync(connection, `SELECT COL FROM ${tableName}`);

          // Then Result should contain 500000 TRUE and 500000 FALSE values
          const trueCount = rows.filter((row) => row.COL === true).length;
          expect(trueCount).toBe(HALF);
          expect(rows.length - trueCount).toBe(HALF);
        } finally {
          await executeAsync(connection, `DROP TABLE IF EXISTS ${tableName}`);
        }
      });
    });
  });

  describe('fetchAsString', () => {
    it('should render booleans as upper-case strings', async () => {
      const { rows } = await executeAsync(
        connection,
        'SELECT TRUE::BOOLEAN AS C1, FALSE::BOOLEAN AS C2',
        { fetchAsString: ['Boolean'] },
      );
      expect(Object.values(rows[0])).toEqual(['TRUE', 'FALSE']);
    });

    it("should render a NULL BOOLEAN cell as the string 'NULL'", async () => {
      const { rows } = await executeAsync(connection, 'SELECT NULL::BOOLEAN', {
        fetchAsString: ['Boolean'],
      });
      expect(Object.values(rows[0])).toEqual(['NULL']);
    });

    it('should render a NULL BOOLEAN cell as null when representNullAsStringNull is disabled', async () => {
      await withNullPreservingConnection(snowflake, async (nullPreservingConnection) => {
        const { rows } = await executeAsync(nullPreservingConnection, 'SELECT NULL::BOOLEAN', {
          fetchAsString: ['Boolean'],
        });
        expect(Object.values(rows[0])).toEqual([null]);
      });
    });
  });
});
