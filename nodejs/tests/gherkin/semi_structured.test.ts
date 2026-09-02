import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import type { Connection } from '../types/sdk-types.js';
import {
  createTestConnection,
  destroyConnectionAsync,
  executeAsync,
  getSnowflakeSDK,
  getStatementColumn,
  isRunningNewDriverWithBD,
} from '../e2e/utils/index.js';

// Gherkin-tracked coverage for tests/definitions/shared/types/semi_structured.feature
// (@nodejs_e2e). Broader OBJECT/MAP/ARRAY getType() and custom-parser coverage not tied to
// these specific scenarios lives in query-data-types-variant.test.ts.
//
// 8 of semi_structured.feature's 15 scenarios are covered here. The rest are deliberately
// left untagged:
// - The 2 table-operation scenarios and the empty-container table round-trip need CREATE
//   TABLE test infrastructure that doesn't exist yet anywhere in nodejs/tests/e2e.
// - Both parameter-binding scenarios are skipped, not just deferred as untested: parameter
//   binding isn't implemented in the new driver at all yet -- `execute()`
//   (nodejs/src/index.ts) only threads `options.sqlText` through to the core, ignoring any
//   `binds` field entirely.
//
// These tests also run against the old driver (e2e-old-driver project). The isArray()/isObject()
// assertions in the cast test branch on BD#4, since the old driver always returns false for those
// predicates on an untyped semi-structured column even though getType() is correct.

describe('Semi-structured type (VARIANT/OBJECT/ARRAY) handling', () => {
  const snowflake = getSnowflakeSDK();
  let connection: Connection;

  beforeAll(async () => {
    connection = createTestConnection(snowflake);
    await connection.connectAsync();
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

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
});
