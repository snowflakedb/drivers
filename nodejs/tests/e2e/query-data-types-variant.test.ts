import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import type { Connection } from '../types/sdk-types.js';
import {
  createTestConnection,
  destroyConnectionAsync,
  executeAsync,
  getStatementColumn,
  getSnowflakeSDK,
  isRunningNewDriverWithBD,
} from './utils/index.js';

describe('Query returning variant data types', () => {
  const snowflake = getSnowflakeSDK();
  let connection: Connection;

  beforeAll(async () => {
    connection = createTestConnection(snowflake);
    await connection.connectAsync();
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  it('returns OBJECT/MAP as Object', async () => {
    // MAP is reported as 'object' because the ENABLE_STRUCTURED_TYPES_IN_CLIENT_RESPONSE session
    // parameter defaults to false. When it is true (and the driver version supports structured
    // types) the server would return it as a structured MAP, and getType() would then report
    // 'map'.
    const { statement, rows } = await executeAsync(
      connection,
      `SELECT
        OBJECT_CONSTRUCT('key', 'value') as OBJECT_COLUMN,
        {'key': 'value'}::MAP(VARCHAR, VARCHAR) as MAP_COLUMN
      `,
    );
    const expectedValue = { key: 'value' };
    const objectColumn = getStatementColumn(statement, 0);
    const mapColumn = getStatementColumn(statement, 1);
    expect(objectColumn.getType()).toBe('object');
    expect(mapColumn.getType()).toBe('object');
    if (isRunningNewDriverWithBD('BD#4')) {
      expect(objectColumn.isObject()).toBe(true);
      expect(mapColumn.isObject()).toBe(true);
    } else {
      expect(objectColumn.isObject()).toBe(false);
      expect(mapColumn.isObject()).toBe(false);
    }
    expect(rows![0].OBJECT_COLUMN).toEqual(expectedValue);
    expect(rows![0].MAP_COLUMN).toEqual(expectedValue);
  });

  it('returns ARRAY as Array', async () => {
    const { statement, rows } = await executeAsync(
      connection,
      'SELECT ARRAY_CONSTRUCT(1, 2, 3) as ARRAY_COLUMN',
    );
    const column = getStatementColumn(statement, 0);
    expect(column.getType()).toBe('array');
    if (isRunningNewDriverWithBD('BD#4')) {
      expect(column.isArray()).toBe(true);
    } else {
      expect(column.isArray()).toBe(false);
    }
    expect(rows![0].ARRAY_COLUMN).toEqual([1, 2, 3]);
  });

  it('parses JSON with undefined, Infinity, NaN as JS types', async () => {
    const { rows } = await executeAsync(
      connection,
      "SELECT parse_json('{a: undefined, b: Infinity, c: NaN, d: [-Infinity, undefined, NaN]}') as VARIANT_COLUMN",
    );
    expect(rows![0].VARIANT_COLUMN).toEqual({
      a: null,
      b: Infinity,
      c: NaN,
      d: [-Infinity, undefined, NaN],
    });
  });

  it('returns VARIANT as the raw text the server sent when fetchAsString is set', async () => {
    const { rows } = await executeAsync(
      connection,
      `SELECT
        parse_json('{"a": 1}'),
        OBJECT_CONSTRUCT('key', 'value'),
        NULL::VARIANT
      `,
      { fetchAsString: ['JSON'] },
    );
    // The old driver skips its whole conversion path for a variant fetched as a
    // string, so its NULL rule never runs and the cell stays null (BD#18).
    const expectedNull = isRunningNewDriverWithBD('BD#18') ? 'NULL' : null;
    expect(Object.values(rows![0])).toEqual(['{\n  "a": 1\n}', { key: 'value' }, expectedNull]);
  });

  it('parses XML as Object', async () => {
    const { rows } = await executeAsync(
      connection,
      "SELECT parse_xml('<root><a>1</a><b>1</b><c><a>1</a></c></root>') as XML_COLUMN",
    );
    expect(rows![0].XML_COLUMN).toEqual({
      root: {
        a: 1,
        b: 1,
        c: { a: 1 },
      },
    });
  });

  // Gherkin-tracked coverage for tests/definitions/shared/types/semi_structured.feature
  // (@nodejs_e2e). 8 of the feature's 15 scenarios are covered here — the rest need CREATE
  // TABLE test infrastructure or parameter binding, neither of which exist yet.
  //
  // These also run against the old driver (e2e-old-driver project); the cast test's
  // isArray()/isObject() assertions branch on BD#4, since the old driver always returns
  // false for those predicates on an untyped semi-structured column even though getType()
  // is correct.
  //
  // Positioned before 'allows to customize JSON/XML parsing' below: those two tests call
  // snowflake.configure() with custom parsers that persist for the rest of this module (see
  // the NOTE above them) and would otherwise make these assertions see 'custom=...' strings
  // instead of parsed values.
  it('should cast semi-structured values to appropriate type', async () => {
    // Given Snowflake client is logged in
    const query = `SELECT
        PARSE_JSON('{"a":1}') AS VARIANT_COL,
        ARRAY_CONSTRUCT(1,2,3) AS ARRAY_COL,
        OBJECT_CONSTRUCT('key','val') AS OBJECT_COL`;

    // When Query "SELECT PARSE_JSON('{\"a\":1}'), ARRAY_CONSTRUCT(1,2,3), OBJECT_CONSTRUCT('key','val')" is executed
    const { statement, rows } = await executeAsync(connection, query);

    // Then All values should be returned as appropriate type
    expect(getStatementColumn(statement, 'VARIANT_COL').isVariant()).toBe(true);
    // BD#4: for an untyped ARRAY/OBJECT column (no structured-types metadata), the old driver's
    // isArray()/isObject() always return false even though getType() reports the right type.
    if (isRunningNewDriverWithBD('BD#4')) {
      expect(getStatementColumn(statement, 'ARRAY_COL').isArray()).toBe(true);
      expect(getStatementColumn(statement, 'OBJECT_COL').isObject()).toBe(true);
    } else {
      expect(getStatementColumn(statement, 'ARRAY_COL').isArray()).toBe(false);
      expect(getStatementColumn(statement, 'OBJECT_COL').isObject()).toBe(false);
    }
    expect(rows![0].VARIANT_COL).toEqual({ a: 1 });
    expect(rows![0].ARRAY_COL).toEqual([1, 2, 3]);
    expect(rows![0].OBJECT_COL).toEqual({ key: 'val' });
  });

  it('should select semi-structured literals', async () => {
    // Given Snowflake client is logged in
    const query = `SELECT PARSE_JSON('{"key":"value"}'), ARRAY_CONSTRUCT(10, 20, 30), OBJECT_CONSTRUCT('a', 1, 'b', 2)`;

    // When Query "SELECT PARSE_JSON('{\"key\":\"value\"}'), ARRAY_CONSTRUCT(10, 20, 30),
    // OBJECT_CONSTRUCT('a', 1, 'b', 2)" is executed
    const { rows } = await executeAsync(connection, query);

    // Then Result should contain the expected values for VARIANT, ARRAY, and OBJECT columns
    expect(Object.values(rows![0])).toEqual([{ key: 'value' }, [10, 20, 30], { a: 1, b: 2 }]);
  });

  it('should select deeply nested semi-structured literals', async () => {
    // Given Snowflake client is logged in
    const query = `SELECT PARSE_JSON('{"a":{"b":[1,2,{"c":true}]}}')`;

    // When Query "SELECT PARSE_JSON('{\"a\":{\"b\":[1,2,{\"c\":true}]}}')" is executed
    const { rows } = await executeAsync(connection, query);

    // Then Result should contain the expected nested value
    expect(Object.values(rows![0])).toEqual([{ a: { b: [1, 2, { c: true }] } }]);
  });

  it('should handle NULL semi-structured values from literals', async () => {
    // Given Snowflake client is logged in
    const query = `SELECT NULL::VARIANT, NULL::OBJECT, NULL::ARRAY`;

    // When Query "SELECT NULL::VARIANT, NULL::OBJECT, NULL::ARRAY" is executed
    const { rows } = await executeAsync(connection, query);

    // Then All columns should return null indicators
    expect(Object.values(rows![0])).toEqual([null, null, null]);
  });

  it('should handle empty JSON containers', async () => {
    // Given Snowflake client is logged in
    const query = `SELECT PARSE_JSON('{}'), ARRAY_CONSTRUCT(), OBJECT_CONSTRUCT()`;

    // When Query "SELECT PARSE_JSON('{}'), ARRAY_CONSTRUCT(), OBJECT_CONSTRUCT()" is executed
    const { rows } = await executeAsync(connection, query);

    // Then Each column should return a valid empty container
    expect(Object.values(rows![0])).toEqual([{}, [], {}]);
  });

  it('should handle empty JSON array literal', async () => {
    // Given Snowflake client is logged in
    const query = `SELECT PARSE_JSON('[]')`;

    // When Query "SELECT PARSE_JSON('[]')" is executed
    const { rows } = await executeAsync(connection, query);

    // Then Result should be an empty JSON array
    expect(Object.values(rows![0])[0]).toEqual([]);
  });

  it('should handle JSON with unicode content', async () => {
    // Given Snowflake client is logged in
    const query = `SELECT PARSE_JSON('{"greeting":"こんにちは","emoji":"⛄"}')`;

    // When Query returning JSON with unicode characters is executed
    const { rows } = await executeAsync(connection, query);

    // Then Result should preserve the unicode characters
    expect(Object.values(rows![0])[0]).toEqual({ greeting: 'こんにちは', emoji: '⛄' });
  });

  it('should handle JSON with unicode in keys', async () => {
    // Given Snowflake client is logged in
    const query = `SELECT PARSE_JSON('{"名前":"テスト","données":"valeur"}')`;

    // When Query returning JSON with unicode characters in keys is executed
    const { rows } = await executeAsync(connection, query);

    // Then Result should preserve unicode keys and their associated values
    expect(Object.values(rows![0])[0]).toEqual({ 名前: 'テスト', données: 'valeur' });
  });

  // NOTE:
  // We do not clear snowflake.configure() between custom parser tests because:
  //  - vitest provides a fresh import for each test file, so configuration does not persist across files.
  //  - There is no public API to unset or reset the settings done via snowflake.configure().
  //  - As a result, we cannot and do not explicitly tear down or clear custom parsers after the tests.
  it('allows to customize JSON parsing', async () => {
    snowflake.configure({
      jsonColumnVariantParser: (rawColumnValue: string) =>
        `custom=${JSON.stringify(JSON.parse(rawColumnValue))}`,
    });
    const { rows } = await executeAsync(connection, "SELECT parse_json('{a: 1}') as JSON_COLUMN");
    expect(rows![0].JSON_COLUMN).toBe('custom={"a":1}');
  });

  it('allows to customize XML parsing', async () => {
    snowflake.configure({
      xmlColumnVariantParser: (rawColumnValue: string) =>
        `custom=${rawColumnValue.replace(/\s+/g, '')}`,
    });
    const { rows } = await executeAsync(
      connection,
      "SELECT parse_xml('<root><a>1</a></root>') as XML_COLUMN",
    );
    expect(rows![0].XML_COLUMN).toBe('custom=<root><a>1</a></root>');
  });
});
