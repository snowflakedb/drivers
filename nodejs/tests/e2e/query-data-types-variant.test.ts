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
