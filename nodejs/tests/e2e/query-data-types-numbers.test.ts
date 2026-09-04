import BigInteger from 'big-integer';
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

/**
 * Builds one `SELECT` with a column per case, so a table of value variations
 * costs a single round trip instead of one per case.
 */
function selectAll(cases: { expression: string }[]): string {
  return `SELECT ${cases.map(({ expression }, index) => `${expression} AS V${index}`).join(', ')}`;
}

// Old driver wraps BigInt-mode values in a `big-integer` instance; the new driver returns a
// native `bigint` (BD#8).
function isBigIntValue(value: unknown): boolean {
  return isRunningNewDriverWithBD('BD#8')
    ? typeof value === 'bigint'
    : BigInteger.isInstance(value);
}

describe('Query returning number data types', () => {
  const snowflake = getSnowflakeSDK();
  let connection: Connection;

  beforeAll(async () => {
    connection = createTestConnection(snowflake);
    await connection.connectAsync();
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  it('returns fixed-point types as Number', async () => {
    const { statement, rows } = await executeAsync(
      connection,
      `SELECT
        1::NUMBER,
        1::DECIMAL,
        1::NUMERIC,
        1::INT,
        1::INTEGER,
        1::BIGINT,
        1::SMALLINT,
        1::TINYINT,
        1::BYTEINT
      `,
    );
    const resultValues = Object.values(rows![0]);
    for (const column of statement.getColumns()!) {
      expect(column.getType()).toBe('fixed');
      expect(column.isNumber()).toBe(true);
      expect(resultValues[column.getIndex()]).toBe(1);
    }
  });

  it('exposes precision and scale of fixed-point types', async () => {
    const { statement } = await executeAsync(
      connection,
      `SELECT
        1::NUMBER(10,3),
        1::DECIMAL(10,3),
        1::NUMERIC(10,3)`,
    );
    for (const column of statement.getColumns()!) {
      expect(column.getPrecision()).toBe(10);
      expect(column.getScale()).toBe(3);
    }
  });

  it('returns scaled fixed-point values as Number', async () => {
    const cases = [
      { expression: '1.25::NUMBER(10,2)', expected: 1.25 },
      { expression: '3.14', expected: 3.14 },
      { expression: '1.50::NUMBER(10,2)', expected: 1.5 },
      { expression: '0.005::NUMBER(10,3)', expected: 0.005 },
      { expression: '-1.25::NUMBER(10,2)', expected: -1.25 },
      // High scale, where the unscaled integer needs most of the precision.
      { expression: '1.5::NUMBER(38,23)', expected: 1.5 },
      { expression: '1.5::NUMBER(38,37)', expected: 1.5 },
    ];
    const { rows } = await executeAsync(connection, selectAll(cases));
    expect(Object.values(rows![0])).toEqual(cases.map(({ expected }) => expected));
  });

  it('returns fixed-point values across integer widths as Number', async () => {
    const cases = [
      { expression: '7::NUMBER(2,0)', expected: 7 },
      { expression: '1234::NUMBER(4,0)', expected: 1234 },
      { expression: '123456789::NUMBER(9,0)', expected: 123456789 },
      { expression: '123456789012345678::NUMBER(18,0)', expected: 123456789012345680 },
    ];
    const { rows } = await executeAsync(connection, selectAll(cases));
    expect(Object.values(rows![0])).toEqual(cases.map(({ expected }) => expected));
  });

  // Past these magnitudes a Number is no longer exact, and core switches its
  // storage from i64 to Decimal128.
  it('returns values at the i64 storage boundary as Number', async () => {
    const cases = [
      { expression: '9223372036854775807', expected: 9223372036854776000 },
      { expression: '9223372036854775808', expected: 9223372036854776000 },
      { expression: '99999999999999999999999999999999999999', expected: 1e38 },
      { expression: '-99999999999999999999999999999999999999', expected: -1e38 },
      {
        expression: '123456789012345678901234567890.12::NUMBER(38,2)',
        expected: 1.2345678901234568e29,
      },
    ];
    const { rows } = await executeAsync(connection, selectAll(cases));
    expect(Object.values(rows![0])).toEqual(cases.map(({ expected }) => expected));
  });

  it('returns fixed-point values as text when fetchAsString is set', async () => {
    const beyondSafeInteger = BigInt(Number.MAX_SAFE_INTEGER) + 4n;
    const cases = [
      { expression: '1.25::NUMBER(10,2)', expected: '1.25' },
      { expression: '1.50::NUMBER(10,2)', expected: '1.50' },
      { expression: '123::INT', expected: '123' },
      { expression: '-7::NUMBER(2,0)', expected: '-7' },
      { expression: `${beyondSafeInteger}::INT`, expected: String(beyondSafeInteger) },
      { expression: '1e1::INT', expected: '10' },
      { expression: 'NULL::INT', expected: 'NULL' },
    ];
    const { statement, rows } = await executeAsync(connection, selectAll(cases), {
      fetchAsString: ['Number'],
    });
    const values = Object.values(rows![0]);
    for (const [index, column] of statement.getColumns()!.entries()) {
      expect(column.getType()).toBe('fixed');
      expect(column.isNumber()).toBe(true);
      expect(values[index]).toBe(cases[index].expected);
    }
  });

  it('returns NULL for fixed-point and float-point types', async () => {
    const { rows } = await executeAsync(
      connection,
      `SELECT
        NULL::INT,
        NULL::NUMBER(10,2),
        NULL::FLOAT,
        NULL::DECFLOAT`,
    );
    expect(Object.values(rows![0])).toEqual([null, null, null, null]);
  });

  it('returns fixed-point and float-point columns together in one row', async () => {
    const { rows } = await executeAsync(
      connection,
      `SELECT
        23::INT,
        1.25::NUMBER(10,2),
        1.5::FLOAT`,
    );
    expect(Object.values(rows![0])).toEqual([23, 1.25, 1.5]);
  });

  it('returns float-point types as Number', async () => {
    const { statement, rows } = await executeAsync(
      connection,
      `SELECT
        1.15::FLOAT,
        1.15::DOUBLE,
        1.15::DOUBLE PRECISION,
        1.15::REAL
      `,
    );
    const resultValues = Object.values(rows![0]);
    for (const column of statement.getColumns()!) {
      expect(column.getType()).toBe('real');
      expect(column.isNumber()).toBe(true);
      expect(resultValues[column.getIndex()]).toBe(1.15);
    }
  });

  it('returns float-point values at the edges of the f64 range', async () => {
    const cases = [
      { expression: '1e-300::FLOAT', expected: 1e-300 },
      { expression: '1e300::FLOAT', expected: 1e300 },
      { expression: '-0.0::FLOAT', expected: -0 },
      { expression: '0.1::FLOAT + 0.2::FLOAT', expected: 0.3 },
      // First integer beyond MAX_SAFE_INTEGER.
      { expression: `${Number.MAX_SAFE_INTEGER + 1}::FLOAT`, expected: 9007199254740990 },
    ];
    const { rows } = await executeAsync(connection, selectAll(cases));
    expect(Object.values(rows![0])).toEqual(cases.map(({ expected }) => expected));
  });

  it('returns float-point values as text when fetchAsString is set', async () => {
    const cases = [
      { expression: '1.15::FLOAT', expectedNew: '1.15', expectedOld: '1.15' },
      { expression: '-0.5::FLOAT', expectedNew: '-0.5', expectedOld: '-0.5' },
      { expression: '1e10::FLOAT', expectedNew: '10000000000', expectedOld: '10000000000' },
      { expression: '1.5e5::FLOAT', expectedNew: '150000', expectedOld: '150000' },
      { expression: '1e300::FLOAT', expectedNew: '1e+300', expectedOld: '1e+300' },
      { expression: '1e-300::FLOAT', expectedNew: '1e-300', expectedOld: '1e-300' },
      { expression: '1e-4::FLOAT', expectedNew: '0.0001', expectedOld: '0.0001' },
      { expression: '1e-5::FLOAT', expectedNew: '0.00001', expectedOld: '1e-05' },
      {
        expression: '999999999999999::FLOAT',
        expectedNew: '999999999999999',
        expectedOld: '999999999999999',
      },
      { expression: '1e15::FLOAT', expectedNew: '1000000000000000', expectedOld: '1e+15' },
      { expression: '-1e15::FLOAT', expectedNew: '-1000000000000000', expectedOld: '-1e+15' },
      { expression: `'inf'::FLOAT`, expectedNew: 'inf', expectedOld: 'inf' },
      { expression: `'-inf'::FLOAT`, expectedNew: '-inf', expectedOld: '-inf' },
      { expression: `'NaN'::FLOAT`, expectedNew: 'NaN', expectedOld: 'NaN' },
      { expression: 'NULL::FLOAT', expectedNew: 'NULL', expectedOld: 'NULL' },
    ];
    const { statement, rows } = await executeAsync(connection, selectAll(cases), {
      fetchAsString: ['Number'],
    });
    const values = Object.values(rows![0]);
    const isRunningNewDriver = isRunningNewDriverWithBD('BD#17');
    for (const [index, column] of statement.getColumns()!.entries()) {
      const { expectedNew, expectedOld } = cases[index];
      expect(column.getType()).toBe('real');
      expect(column.isNumber()).toBe(true);
      expect(values[index]).toBe(isRunningNewDriver ? expectedNew : expectedOld);
    }
  });

  // TODO: this test must have warning and telemetry event check
  it('returns large numbers (> Number.MAX_SAFE_INTEGER) with precision loss', async () => {
    const { rows } = await executeAsync(
      connection,
      `SELECT
        9007199254740995 as LARGE_FIXED_COLUMN,
        9007199254740930.13231312::FLOAT as LARGE_FLOAT_COLUMN`,
    );
    const selectedFixedValue = rows![0].LARGE_FIXED_COLUMN as number;
    const selectedFloatValue = rows![0].LARGE_FLOAT_COLUMN as number;
    expect(selectedFixedValue.toString()).toBe('9007199254740996');
    expect(selectedFloatValue.toString()).toBe('9007199254740930');
  });

  it('returns FLOAT special values as JS numbers', async () => {
    const { rows } = await executeAsync(
      connection,
      `SELECT
        'NaN'::FLOAT,
        'inf'::FLOAT,
        '-inf'::FLOAT`,
    );
    const expected = isRunningNewDriverWithBD('BD#9')
      ? [NaN, Infinity, -Infinity]
      : [NaN, NaN, NaN];
    expect(Object.values(rows![0])).toEqual(expected);
  });

  for (const { name, connectionFactory } of [
    {
      name: 'JS_TREAT_INTEGER_AS_BIGINT session parameter',
      connectionFactory: async () => {
        const connection = createTestConnection(snowflake);
        await connection.connectAsync();
        await executeAsync(connection, 'ALTER SESSION SET JS_TREAT_INTEGER_AS_BIGINT = true');
        return connection;
      },
    },
    {
      name: 'jsTreatIntegerAsBigInt connection parameter',
      connectionFactory: async () => {
        const connection = createTestConnection(snowflake, {
          jsTreatIntegerAsBigInt: true,
        });
        await connection.connectAsync();
        return connection;
      },
    },
  ]) {
    it(`returns integer as BigInt instance when ${name} is set`, async () => {
      const bigIntConnection = await connectionFactory();
      try {
        const { statement, rows } = await executeAsync(
          bigIntConnection,
          'SELECT 90071992547409954434323 as INT_COLUMN',
        );
        const resultColumn = getStatementColumn(statement, 0);
        const selectedValue = rows![0].INT_COLUMN;
        expect(resultColumn.getType()).toBe('fixed');
        expect(resultColumn.isNumber()).toBe(true);
        expect(isBigIntValue(selectedValue)).toBe(true);
        expect(String(selectedValue)).toBe('90071992547409954434323');
      } finally {
        await destroyConnectionAsync(bigIntConnection);
      }
    });
  }

  it('returns DECFLOAT literals correctly formatted', async () => {
    const cases: { literal: string; expected: string | null }[] = [
      { literal: "'1.5'", expected: '1.5' },
      { literal: "'1.500'", expected: '1.5' },
      { literal: "'0'", expected: '0' },
      { literal: "'-0'", expected: '0' },
      { literal: "'123'", expected: '123' },
      { literal: "'-123'", expected: '-123' },
      { literal: "'100'", expected: '100' },
      { literal: "'0.00001'", expected: '0.00001' },
      { literal: "'1e10'", expected: '10000000000' },
      { literal: "'1e-10'", expected: '0.0000000001' },
      { literal: "'-1.5e-10'", expected: '-0.00000000015' },
      {
        literal: "'99999999999999999999999999999999999999'",
        expected: '99999999999999999999999999999999999999',
      },
      // Both sides of the plain/scientific switch, which turns on the adjusted
      // exponent reaching the precision (38) and is not documented.
      { literal: "'1e37'", expected: '10000000000000000000000000000000000000' },
      { literal: "'1e38'", expected: '1e38' },
      { literal: "'1e-36'", expected: '0.000000000000000000000000000000000001' },
      { literal: "'1e-37'", expected: '0.0000000000000000000000000000000000001' },
      { literal: "'1e-38'", expected: '1e-38' },
      // 38 digits with an adjusted exponent of 0: plain, and no "e0" suffix.
      {
        literal: "'1.2345678901234567890123456789012345678'",
        expected: '1.2345678901234567890123456789012345678',
      },
      // The 39th digit is rounded off by the server, not by the driver.
      {
        literal: "'1.23456789012345678901234567890123456789'",
        expected: '1.2345678901234567890123456789012345679',
      },
      { literal: "'1000000000000000000000000000000000000000'", expected: '1e39' },
      // Documented exponent range: -16383 to 16384.
      { literal: "'1e16384'", expected: '1e16384' },
      { literal: "'1e-16383'", expected: '1e-16383' },
      { literal: "'1e-16384'", expected: '0' },
      {
        literal: "'1.2345678901234567890123456789012345678e16384'",
        expected: '1.2345678901234567890123456789012345678e16384',
      },
    ];
    const columns = cases.map(({ literal }, i) => `${literal}::DECFLOAT AS V${i}`);
    const { statement, rows } = await executeAsync(connection, `SELECT ${columns.join(', ')}`);
    for (const column of statement.getColumns()!) {
      expect(column.getType()).toBe('decfloat');
      if (isRunningNewDriverWithBD('BD#6')) {
        // @ts-ignore TODO: remove once the test runner's Column type is the new driver's own,
        // not the old driver's (see utils/index.ts TODO)
        expect(column.isDecfloat()).toBe(true);
      }
    }
    expect(Object.values(rows![0])).toEqual(cases.map((c) => c.expected));
  });

  it('returns DECFLOAT above the maximum exponent', async () => {
    const { rows } = await executeAsync(connection, `SELECT '1e16385'::DECFLOAT`);
    const value = Object.values(rows![0])[0];
    if (isRunningNewDriverWithBD('BD#7')) {
      expect(value).toBe('1e16385');
    } else {
      expect(value).toBe('10e16384');
    }
  });

  it('renders a NULL DECFLOAT as text when fetchAsString is set', async () => {
    const { rows } = await executeAsync(connection, "SELECT '1.5'::DECFLOAT, NULL::DECFLOAT", {
      fetchAsString: ['String', 'Number'],
    });
    const expectedNull = isRunningNewDriverWithBD('BD#18') ? 'NULL' : null;
    expect(Object.values(rows![0])).toEqual(['1.5', expectedNull]);
  });
});

describe('Query returning BigInt data types', () => {
  const snowflake = getSnowflakeSDK();
  let connection: Connection;

  beforeAll(async () => {
    connection = createTestConnection(snowflake, { jsTreatIntegerAsBigInt: true });
    await connection.connectAsync();
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  it('returns integers as exact BigInt instances', async () => {
    const cases = [
      { expression: '7::NUMBER(2,0)', expected: '7' },
      { expression: '9223372036854775807', expected: '9223372036854775807' },
      { expression: '9223372036854775808', expected: '9223372036854775808' },
      {
        expression: '99999999999999999999999999999999999999',
        expected: '99999999999999999999999999999999999999',
      },
      {
        expression: '-99999999999999999999999999999999999999',
        expected: '-99999999999999999999999999999999999999',
      },
    ];
    const { statement, rows } = await executeAsync(connection!, selectAll(cases));
    const values = Object.values(rows![0]);

    expect(statement.getColumns()!.map((column) => column.getType())).toEqual(
      cases.map(() => 'fixed'),
    );
    expect(values.map((value) => isBigIntValue(value))).toEqual(cases.map(() => true));
    expect(values.map(String)).toEqual(cases.map(({ expected }) => expected));
  });

  it('leaves scaled NUMBER(38,2) as a Number', async () => {
    const { rows } = await executeAsync(
      connection!,
      'SELECT 123456789012345678901234567890.12::NUMBER(38,2)',
    );
    const value = Object.values(rows![0])[0];
    expect(isBigIntValue(value)).toBe(false);
    expect(value).toBe(1.2345678901234568e29);
  });

  it('returns NULL as null, not a BigInt', async () => {
    const { rows } = await executeAsync(connection!, 'SELECT NULL::INT');
    expect(Object.values(rows![0])[0]).toBeNull();
  });

  describe('fetchAsString', () => {
    it('returns text rather than a BigInt', async () => {
      const { rows } = await executeAsync(connection!, 'SELECT 90071992547409954434323', {
        fetchAsString: ['Number'],
      });
      expect(Object.values(rows![0])).toEqual(['90071992547409954434323']);
    });
  });
});
