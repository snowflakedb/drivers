import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import type { Connection } from '../types/sdk-types.js';
import { isBigIntValue } from './query/utils.js';
import {
  createTestConnection,
  destroyConnectionAsync,
  executeAsync,
  getStatementColumn,
  isRunningNewDriverWithBD,
} from './utils/index.js';

/**
 * Builds one `SELECT` with a column per case, so a table of value variations
 * costs a single round trip instead of one per case.
 */
function selectAll(cases: { expression: string }[]): string {
  return `SELECT ${cases.map(({ expression }, index) => `${expression} AS V${index}`).join(', ')}`;
}

describe('Query returning number data types', () => {
  let connection: Connection;

  beforeAll(async () => {
    connection = createTestConnection();
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
        1::NUMERIC
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

  // Past this magnitude a Number is no longer exact, and core switches its
  // storage from i64 to Decimal128.
  it('returns a scaled fixed-point value beyond the i64 boundary as Number', async () => {
    const { rows } = await executeAsync(
      connection,
      'SELECT 123456789012345678901234567890.12::NUMBER(38,2)',
    );
    expect(Object.values(rows![0])).toEqual([1.2345678901234568e29]);
  });

  it('returns fixed-point values as text when fetchAsString is set', async () => {
    const cases = [
      { expression: '1.25::NUMBER(10,2)', expected: '1.25' },
      { expression: '1.50::NUMBER(10,2)', expected: '1.50' },
      { expression: '-7::NUMBER(2,0)', expected: '-7' },
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
        NULL::FLOAT`,
    );
    expect(Object.values(rows![0])).toEqual([null, null, null]);
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

  it('returns large float values (> Number.MAX_SAFE_INTEGER) with precision loss', async () => {
    const { rows } = await executeAsync(
      connection,
      'SELECT 9007199254740930.13231312::FLOAT as LARGE_FLOAT_COLUMN',
    );
    const selectedFloatValue = rows![0].LARGE_FLOAT_COLUMN as number;
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
        const connection = createTestConnection();
        await connection.connectAsync();
        await executeAsync(connection, 'ALTER SESSION SET JS_TREAT_INTEGER_AS_BIGINT = true');
        return connection;
      },
    },
    {
      name: 'jsTreatIntegerAsBigInt connection parameter',
      connectionFactory: async () => {
        const connection = createTestConnection({
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
});

describe('Query returning BigInt data types', () => {
  let connection: Connection;

  beforeAll(async () => {
    connection = createTestConnection({ jsTreatIntegerAsBigInt: true });
    await connection.connectAsync();
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
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
});
