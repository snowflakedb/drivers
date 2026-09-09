import { afterAll, afterEach, beforeAll, describe, expect, it } from 'vitest';
import type { Connection } from '../../../types/sdk-types.js';
import { createLiveConnection, createTemporaryTable } from '../../utils/fixtures.js';
import {
  destroyConnectionAsync,
  executeAsync,
  getStatementColumn,
  isRunningNewDriverWithBD,
  NOT_IMPLEMENTED_IN_NEW_DRIVER,
  snowflake,
  resetGlobalConfig,
} from '../../utils/index.js';
import { createLiveNullPreservingConnection } from '../utils.js';

describe('semi-structured data type', () => {
  let connection: Connection;

  beforeAll(async () => {
    connection = await createLiveConnection({}, false);
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  describe('tests/definitions/shared/types/semi_structured.feature', () => {
    it('should cast semi-structured values to appropriate type', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT PARSE_JSON('{\"a\":1}'), ARRAY_CONSTRUCT(1,2,3), OBJECT_CONSTRUCT('key','val')" is executed
      const { statement, rows } = await executeAsync(
        connection,
        `SELECT
            PARSE_JSON('{"a":1}') AS VARIANT_COL,
            ARRAY_CONSTRUCT(1,2,3) AS ARRAY_COL,
            OBJECT_CONSTRUCT('key','val') AS OBJECT_COL`,
      );

      // Then All values should be returned as appropriate type
      const variantColumn = getStatementColumn(statement, 'VARIANT_COL');
      const arrayColumn = getStatementColumn(statement, 'ARRAY_COL');
      const objectColumn = getStatementColumn(statement, 'OBJECT_COL');
      expect(variantColumn.getType()).toBe('variant');
      expect(arrayColumn.getType()).toBe('array');
      expect(objectColumn.getType()).toBe('object');
      expect(variantColumn.isVariant()).toBe(true);
      if (isRunningNewDriverWithBD('BD#4')) {
        expect(arrayColumn.isArray()).toBe(true);
        expect(objectColumn.isObject()).toBe(true);
      } else {
        expect(arrayColumn.isArray()).toBe(false);
        expect(objectColumn.isObject()).toBe(false);
      }
      expect(rows[0].VARIANT_COL).toEqual({ a: 1 });
      expect(rows[0].ARRAY_COL).toEqual([1, 2, 3]);
      expect(rows[0].OBJECT_COL).toEqual({ key: 'val' });
    });

    it('should select semi-structured literals', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT PARSE_JSON('{\"key\":\"value\"}'), ARRAY_CONSTRUCT(10, 20, 30), OBJECT_CONSTRUCT('a', 1, 'b', 2)" is executed
      const { statement, rows } = await executeAsync(
        connection,
        `SELECT PARSE_JSON('{"key":"value"}'), ARRAY_CONSTRUCT(10, 20, 30), OBJECT_CONSTRUCT('a', 1, 'b', 2)`,
      );

      // Then Result should contain the expected values for VARIANT, ARRAY, and OBJECT columns
      expect(getStatementColumn(statement, 0).getType()).toBe('variant');
      expect(getStatementColumn(statement, 1).getType()).toBe('array');
      expect(getStatementColumn(statement, 2).getType()).toBe('object');
      expect(Object.values(rows[0])).toEqual([{ key: 'value' }, [10, 20, 30], { a: 1, b: 2 }]);
    });

    it('should select deeply nested semi-structured literals', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT PARSE_JSON('{\"a\":{\"b\":[1,2,{\"c\":true}]}}')" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT PARSE_JSON('{"a":{"b":[1,2,{"c":true}]}}')`,
      );

      // Then Result should contain the expected nested value
      expect(Object.values(rows[0])).toEqual([{ a: { b: [1, 2, { c: true }] } }]);
    });

    it('should handle NULL semi-structured values from literals', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT NULL::VARIANT, NULL::OBJECT, NULL::ARRAY" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT NULL::VARIANT, NULL::OBJECT, NULL::ARRAY`,
      );

      // Then All columns should return null indicators
      expect(Object.values(rows[0])).toEqual([null, null, null]);
    });

    it('should select semi-structured values from table', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And Table with VARIANT, OBJECT, and ARRAY columns exists with JSON values
      const tableName = await createTemporaryTable(
        connection,
        'ID NUMBER, VARIANT_COL VARIANT, OBJECT_COL OBJECT, ARRAY_COL ARRAY',
      );
      await executeAsync(
        connection,
        `INSERT INTO ${tableName} (ID, VARIANT_COL, OBJECT_COL, ARRAY_COL)
           SELECT 1, PARSE_JSON('{"key":"value"}'), OBJECT_CONSTRUCT('a', 1, 'b', 2), ARRAY_CONSTRUCT(10, 20, 30)`,
      );

      // When Query "SELECT * FROM <table>" is executed
      const { statement, rows } = await executeAsync(
        connection,
        `SELECT VARIANT_COL, OBJECT_COL, ARRAY_COL FROM ${tableName} ORDER BY ID`,
      );

      // Then Data should contain the expected semi-structured values
      expect(getStatementColumn(statement, 'VARIANT_COL').getType()).toBe('variant');
      expect(getStatementColumn(statement, 'OBJECT_COL').getType()).toBe('object');
      expect(getStatementColumn(statement, 'ARRAY_COL').getType()).toBe('array');
      expect(rows[0].VARIANT_COL).toEqual({ key: 'value' });
      expect(rows[0].OBJECT_COL).toEqual({ a: 1, b: 2 });
      expect(rows[0].ARRAY_COL).toEqual([10, 20, 30]);
    });

    it('should handle NULL semi-structured values from table', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And Table with VARIANT column exists containing NULLs and values
      const tableName = await createTemporaryTable(connection, 'ID NUMBER, VARIANT_COL VARIANT');
      await executeAsync(
        connection,
        `INSERT INTO ${tableName} (ID, VARIANT_COL)
           SELECT 1, NULL::VARIANT
           UNION ALL SELECT 2, PARSE_JSON('{"a":1}')
           UNION ALL SELECT 3, NULL::VARIANT`,
      );

      // When Query "SELECT * FROM <table>" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT VARIANT_COL FROM ${tableName} ORDER BY ID`,
      );

      // Then Result should contain [NULL, {"a":1}, NULL]
      expect(rows.map((row) => row.VARIANT_COL)).toEqual([null, { a: 1 }, null]);
    });

    it('should handle empty JSON containers', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT PARSE_JSON('{}'), ARRAY_CONSTRUCT(), OBJECT_CONSTRUCT()" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT PARSE_JSON('{}'), ARRAY_CONSTRUCT(), OBJECT_CONSTRUCT()`,
      );

      // Then Each column should return a valid empty container
      expect(Object.values(rows[0])).toEqual([{}, [], {}]);
    });

    it('should handle empty JSON array literal', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT PARSE_JSON('[]')" is executed
      const { rows } = await executeAsync(connection, `SELECT PARSE_JSON('[]')`);

      // Then Result should be an empty JSON array
      expect(Object.values(rows[0])[0]).toEqual([]);
    });

    it('should round-trip empty JSON containers through a table', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And Table with VARIANT, OBJECT, and ARRAY columns exists with empty containers
      const tableName = await createTemporaryTable(
        connection,
        'ID NUMBER, VARIANT_COL VARIANT, OBJECT_COL OBJECT, ARRAY_COL ARRAY',
      );
      await executeAsync(
        connection,
        `INSERT INTO ${tableName} (ID, VARIANT_COL, OBJECT_COL, ARRAY_COL)
           SELECT 1, PARSE_JSON('{}'), OBJECT_CONSTRUCT(), ARRAY_CONSTRUCT()`,
      );

      // When Query "SELECT * FROM <table>" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT VARIANT_COL, OBJECT_COL, ARRAY_COL FROM ${tableName} ORDER BY ID`,
      );

      // Then All columns should return valid empty containers
      expect(rows[0].VARIANT_COL).toEqual({});
      expect(rows[0].OBJECT_COL).toEqual({});
      expect(rows[0].ARRAY_COL).toEqual([]);
    });

    it('should handle JSON with unicode content', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query returning JSON with unicode characters is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT PARSE_JSON('{"greeting":"こんにちは","emoji":"⛄"}')`,
      );

      // Then Result should preserve the unicode characters
      expect(Object.values(rows[0])[0]).toEqual({ greeting: 'こんにちは', emoji: '⛄' });
    });

    it('should handle JSON with unicode in keys', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query returning JSON with unicode characters in keys is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT PARSE_JSON('{"名前":"テスト","données":"valeur"}')`,
      );

      // Then Result should preserve unicode keys and their associated values
      expect(Object.values(rows[0])[0]).toEqual({ 名前: 'テスト', données: 'valeur' });
    });

    describe('parameter binding', () => {
      it('should select variant using parameter binding', async () => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT PARSE_JSON(?)" is executed with bound JSON string '{"bound":true}'
        const { rows } = await executeAsync(connection, `SELECT PARSE_JSON(?)`, {
          binds: ['{"bound":true}'],
        });

        // Then Result should contain a value with "bound" key
        expect(Object.values(rows[0])[0]).toEqual({ bound: true });
      });

      it('should select NULL variant using parameter binding', async () => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT PARSE_JSON(?)" is executed with bound NULL value
        const { rows } = await executeAsync(connection, `SELECT PARSE_JSON(?)`, { binds: [null] });

        // Then Result should be NULL
        expect(Object.values(rows[0])[0]).toBeNull();
      });

      it('should insert variant using parameter binding', async () => {
        // Given Snowflake client is logged in
        void connection;

        // And Table with VARIANT column exists
        const tableName = await createTemporaryTable(connection, 'ID NUMBER, VARIANT_COL VARIANT');
        // When JSON values are inserted using parameter binding via PARSE_JSON(?)
        await executeAsync(
          connection,
          `INSERT INTO ${tableName} (ID, VARIANT_COL)
             SELECT 1, PARSE_JSON(?)
             UNION ALL SELECT 2, PARSE_JSON(?)`,
          { binds: ['{"bound":true}', '{"a":1}'] },
        );

        // Then SELECT should return the inserted JSON values
        const { rows } = await executeAsync(
          connection,
          `SELECT VARIANT_COL FROM ${tableName} ORDER BY ID`,
        );
        expect(rows.map((row) => row.VARIANT_COL)).toEqual([{ bound: true }, { a: 1 }]);
      });
    });

    describe.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)('multiple chunks', () => {
      const rowCount = 20_000;

      it('should download semi-structured data in multiple chunks', async () => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT OBJECT_CONSTRUCT('id', seq8()) AS obj FROM TABLE(GENERATOR(ROWCOUNT => 20000)) v ORDER BY 1" is executed
        const { rows } = await executeAsync(
          connection,
          `SELECT OBJECT_CONSTRUCT('id', seq8()) AS obj FROM TABLE(GENERATOR(ROWCOUNT => ${rowCount})) v ORDER BY 1`,
        );

        // Then All 20000 rows should be fetched and each should contain a value with "id" key
        expect(rows).toHaveLength(rowCount);
        expect(rows.every((row) => (row.OBJ as Record<string, unknown>)?.id !== undefined)).toBe(
          true,
        );
      });
    });
  });

  describe('fetchAsString', () => {
    it('should return VARIANT as the raw text the server sent when fetchAsString is set', async () => {
      const { rows } = await executeAsync(
        connection,
        `SELECT parse_json('{"a": 1}') AS PARSED, OBJECT_CONSTRUCT('key', 'value') AS CONSTRUCTED`,
        { fetchAsString: ['JSON'] },
      );
      // The 'JSON' token stringifies the VARIANT column but not the OBJECT column, so
      // OBJECT_CONSTRUCT still comes back as a parsed object.
      expect(Object.values(rows[0])).toEqual(['{\n  "a": 1\n}', { key: 'value' }]);
    });

    it("should render a NULL VARIANT cell as the string 'NULL' when fetchAsString is set", async () => {
      const { rows } = await executeAsync(connection, `SELECT NULL::VARIANT`, {
        fetchAsString: ['JSON'],
      });
      // The old driver skips its whole conversion path for a variant fetched as a
      // string, so its NULL rule never runs and the cell stays null (BD#18).
      const expectedNull = isRunningNewDriverWithBD('BD#18') ? 'NULL' : null;
      expect(Object.values(rows[0])).toEqual([expectedNull]);
    });

    it('should render a NULL VARIANT cell as null when representNullAsStringNull is disabled', async () => {
      const nullPreservingConnection = await createLiveNullPreservingConnection();
      const { rows } = await executeAsync(nullPreservingConnection, `SELECT NULL::VARIANT`, {
        fetchAsString: ['JSON'],
      });
      expect(Object.values(rows[0])).toEqual([null]);
    });
  });

  describe('MAP columns', () => {
    it('should report a MAP column as object', async () => {
      const { statement, rows } = await executeAsync(
        connection,
        `SELECT {'key': 'value'}::MAP(VARCHAR, VARCHAR) AS MAP_COLUMN`,
      );
      const mapColumn = getStatementColumn(statement, 0);
      expect(mapColumn.getType()).toBe('object');
      if (isRunningNewDriverWithBD('BD#4')) {
        expect(mapColumn.isObject()).toBe(true);
      } else {
        expect(mapColumn.isObject()).toBe(false);
      }
      expect(rows[0].MAP_COLUMN).toEqual({ key: 'value' });
    });
  });

  describe('Parsing', () => {
    it('should parse JSON with undefined, Infinity, and NaN as JS types', async () => {
      const { rows } = await executeAsync(
        connection,
        "SELECT parse_json('{a: undefined, b: Infinity, c: NaN, d: [-Infinity, undefined, NaN]}') as VARIANT_COLUMN",
      );
      expect(rows[0].VARIANT_COLUMN).toEqual({
        a: null,
        b: Infinity,
        c: NaN,
        d: [-Infinity, undefined, NaN],
      });
    });

    it('should parse XML as an object', async () => {
      const { rows } = await executeAsync(
        connection,
        "SELECT parse_xml('<root><a>1</a><b>1</b><c><a>1</a></c></root>') as XML_COLUMN",
      );
      expect(rows[0].XML_COLUMN).toEqual({
        root: {
          a: 1,
          b: 1,
          c: { a: 1 },
        },
      });
    });

    describe('custom parsers', () => {
      afterEach(resetGlobalConfig);

      it('should allow customizing JSON parsing', async () => {
        snowflake.configure({
          jsonColumnVariantParser: (rawColumnValue: string) =>
            `custom=${JSON.stringify(JSON.parse(rawColumnValue))}`,
        });
        const { rows } = await executeAsync(
          connection,
          "SELECT parse_json('{a: 1}') as JSON_COLUMN",
        );
        expect(rows[0].JSON_COLUMN).toBe('custom={"a":1}');
      });

      it('should allow customizing XML parsing', async () => {
        snowflake.configure({
          xmlColumnVariantParser: (rawColumnValue: string) =>
            `custom=${rawColumnValue.replace(/\s+/g, '')}`,
        });
        const { rows } = await executeAsync(
          connection,
          "SELECT parse_xml('<root><a>1</a></root>') as XML_COLUMN",
        );
        expect(rows[0].XML_COLUMN).toBe('custom=<root><a>1</a></root>');
      });
    });
  });
});
