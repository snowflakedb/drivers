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

// Snowflake resolves backslash escapes such as `\t` and `\u26c4` inside single-quoted
// constants, so `literalSql` and `expected` differ for those rows. A Snowflake Unicode escape
// takes exactly four hex digits, which is why the G clef (U+1D11E) is passed as the character
// itself.
const CORNER_CASES: { name: string; literalSql: string; expected: string | null }[] = [
  { name: 'empty string', literalSql: "''", expected: '' },
  { name: 'single character', literalSql: "'X'", expected: 'X' },
  { name: 'whitespace only', literalSql: "'   '", expected: '   ' },
  { name: 'tab character', literalSql: "'\\t'", expected: '\t' },
  { name: 'newline', literalSql: "'\\n'", expected: '\n' },
  { name: 'unicode snowman', literalSql: "'\\u26c4'", expected: '\u26c4' },
  {
    name: 'japanese characters',
    literalSql: "'日本語テスト'",
    expected: '日本語テスト',
  },
  { name: 'escaped single quote', literalSql: "'\\''", expected: "'" },
  { name: 'escaped backslash', literalSql: "'\\\\'", expected: '\\' },
  { name: 'NULL', literalSql: 'NULL', expected: null },
  {
    name: 'combining diacritical mark',
    literalSql: "'y\u0306es'",
    expected: 'y\u0306es',
  },
  { name: 'surrogate pair', literalSql: "'\u{1D11E}'", expected: '\u{1D11E}' },
];

describe('STRING data type', () => {
  const snowflake = getSnowflakeSDK();
  let connection: Connection;

  beforeAll(async () => {
    connection = createTestConnection(snowflake);
    await connection.connectAsync();
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  describe('tests/definitions/shared/types/string.feature', () => {
    it.each([
      'VARCHAR',
      'STRING',
      'TEXT',
      'VARCHAR2',
      'NVARCHAR',
      'NVARCHAR2',
      'CHAR VARYING',
      'NCHAR VARYING',
      'CHAR(32)',
      'CHARACTER(32)',
      'NCHAR(32)',
    ])(
      'should cast string values to appropriate type for string and synonyms (%s)',
      async (type) => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT 'hello'::<type>, 'Hello World'::<type>, '日本語テスト'::<type>" is executed
        const { statement, rows } = await executeAsync(
          connection,
          `SELECT 'hello'::${type}, 'Hello World'::${type}, '日本語テスト'::${type}`,
        );

        // Then All values should be returned as appropriate type
        for (const index of [0, 1, 2]) {
          const column = getStatementColumn(statement, index);
          expect(column.getType()).toBe('text');
          expect(column.isString()).toBe(true);
        }
        expect(Object.values(rows[0])).toEqual(['hello', 'Hello World', '日本語テスト']);
      },
    );

    it('should select hardcoded string literals', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT 'hello' AS str1, 'Hello World' AS str2, 'Snowflake Driver Test' AS str3" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT 'hello' AS str1, 'Hello World' AS str2, 'Snowflake Driver Test' AS str3`,
      );

      // Then the result should contain:
      expect(Object.values(rows[0])).toEqual(['hello', 'Hello World', 'Snowflake Driver Test']);
    });

    it('should select string literals with corner case values', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query selecting corner case string literals is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT ${CORNER_CASES.map(({ literalSql }, index) => `${literalSql} AS C${index}`).join(', ')}`,
      );

      // Then the result should contain expected corner case string values
      CORNER_CASES.forEach(({ name, expected }, index) => {
        expect(rows[0][`C${index}`], name).toBe(expected);
      });
    });

    it('should select hardcoded string values from table', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And A temporary table with VARCHAR column is created
      const tableName = randomizeName('STRING_TEST');
      await executeAsync(
        connection,
        `CREATE OR REPLACE TEMPORARY TABLE ${tableName} (ID NUMBER, VAL VARCHAR)`,
      );
      try {
        // And The table is populated with string values
        await executeAsync(
          connection,
          `INSERT INTO ${tableName} (ID, VAL)
           VALUES (1, 'hello'), (2, 'Hello World'), (3, 'Snowflake Driver Test')`,
        );

        // When Query "SELECT * FROM {table}" is executed
        const { rows } = await executeAsync(connection, `SELECT VAL FROM ${tableName} ORDER BY ID`);

        // Then the result should contain the inserted hardcoded string values
        expect(rows.map((row) => row.VAL)).toEqual([
          'hello',
          'Hello World',
          'Snowflake Driver Test',
        ]);
      } finally {
        await executeAsync(connection, `DROP TABLE IF EXISTS ${tableName}`);
      }
    });

    it('should select corner case string values from table', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And A temporary table with VARCHAR column is created
      const tableName = randomizeName('STRING_CORNER_CASE_TEST');
      await executeAsync(
        connection,
        `CREATE OR REPLACE TEMPORARY TABLE ${tableName} (ID NUMBER, VAL VARCHAR)`,
      );
      try {
        // And The table is populated with corner case string values
        await executeAsync(
          connection,
          `INSERT INTO ${tableName} (ID, VAL) VALUES ${CORNER_CASES.map(
            ({ literalSql }, index) => `(${index}, ${literalSql})`,
          ).join(', ')}`,
        );

        // When Query "SELECT * FROM {table}" is executed
        const { rows } = await executeAsync(connection, `SELECT VAL FROM ${tableName} ORDER BY ID`);

        // Then the result should contain the inserted corner case string values
        expect(rows.map((row) => row.VAL)).toEqual(CORNER_CASES.map(({ expected }) => expected));
      } finally {
        await executeAsync(connection, `DROP TABLE IF EXISTS ${tableName}`);
      }
    });

    describe.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)('parameter binding', () => {
      it('should select string literals using parameter binding', async () => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT ?::VARCHAR, ?::VARCHAR, ?::VARCHAR" is executed with bound string values ['hello', 'Hello World', '日本語テスト']
        // Without the aliases all three columns are named `?::VARCHAR` and collapse into one
        // key in object row mode.
        const { rows } = await executeAsync(
          connection,
          'SELECT ?::VARCHAR AS COL1, ?::VARCHAR AS COL2, ?::VARCHAR AS COL3',
          { binds: ['hello', 'Hello World', '日本語テスト'] },
        );

        // Then the result should contain:
        expect(Object.values(rows[0])).toEqual(['hello', 'Hello World', '日本語テスト']);
      });

      it.each(CORNER_CASES)(
        'should select corner case string values using parameter binding ($name)',
        async ({ expected }) => {
          // Given Snowflake client is logged in
          void connection;

          // When Query "SELECT ?::VARCHAR" is executed with each corner case string value bound
          const { rows } = await executeAsync(connection, 'SELECT ?::VARCHAR', {
            binds: [expected],
          });

          // Then the result should match the bound corner case value
          expect(Object.values(rows[0])).toEqual([expected]);
        },
      );

      it('should insert and select back hardcoded string values using parameter binding', async () => {
        // Given Snowflake client is logged in
        void connection;

        // And A temporary table with VARCHAR column is created
        const tableName = randomizeName('STRING_BIND_TEST');
        await executeAsync(
          connection,
          `CREATE OR REPLACE TEMPORARY TABLE ${tableName} (VAL VARCHAR)`,
        );
        try {
          // When String value 'Test binding value 日本語' is inserted using parameter binding
          await executeAsync(connection, `INSERT INTO ${tableName} (VAL) VALUES (?)`, {
            binds: ['Test binding value 日本語'],
          });

          // And Query "SELECT * FROM {table}" is executed
          const { rows } = await executeAsync(connection, `SELECT VAL FROM ${tableName}`);

          // Then the result should contain the bound string value 'Test binding value 日本語'
          expect(rows.map((row) => row.VAL)).toEqual(['Test binding value 日本語']);
        } finally {
          await executeAsync(connection, `DROP TABLE IF EXISTS ${tableName}`);
        }
      });
    });

    describe.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)('multiple chunks', () => {
      it('should download string data in multiple chunks', async () => {
        // 10k rows is past the size at which the server splits a result into more than one chunk.
        const rowCount = 10_000;

        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT seq8() AS id, TO_VARCHAR(seq8()) AS str_val FROM TABLE(GENERATOR(ROWCOUNT => 10000)) v ORDER BY id" is executed
        const { rows } = await executeAsync(
          connection,
          `SELECT seq8() AS id, TO_VARCHAR(seq8()) AS str_val FROM TABLE(GENERATOR(ROWCOUNT => ${rowCount})) v ORDER BY id`,
        );

        // Then there are 10000 rows returned and all string values should match the generated values in order
        expect(rows).toHaveLength(rowCount);
        expect(rows.map((row) => row.STR_VAL)).toEqual(
          Array.from({ length: rowCount }, (_, index) => String(index)),
        );
      });
    });
  });

  describe('fetchAsString', () => {
    it('should return TEXT unchanged', async () => {
      const { rows } = await executeAsync(connection, "SELECT 'a'::TEXT", {
        fetchAsString: ['String'],
      });
      expect(Object.values(rows[0])).toEqual(['a']);
    });

    it("should render a NULL TEXT cell as the string 'NULL'", async () => {
      const { rows } = await executeAsync(connection, 'SELECT NULL::TEXT', {
        fetchAsString: ['String'],
      });
      expect(Object.values(rows[0])).toEqual(['NULL']);
    });

    it.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)(
      'should render a NULL TEXT cell as null when representNullAsStringNull is disabled',
      async () => {
        await withNullPreservingConnection(snowflake, async (nullPreservingConnection) => {
          const { rows } = await executeAsync(nullPreservingConnection, 'SELECT NULL::TEXT', {
            fetchAsString: ['String'],
          });
          expect(Object.values(rows[0])).toEqual([null]);
        });
      },
    );
  });
});
