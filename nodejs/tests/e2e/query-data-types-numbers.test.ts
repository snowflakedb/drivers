import BigInteger from 'big-integer';
import { Connection } from 'snowflake-sdk';
import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import {
  createTestConnection,
  destroyConnectionAsync,
  executeAsync,
  getSnowflakeSDK,
  isRunningNewDriverWithBD,
} from './utils';

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

  // Bug - doesn't work in old driver
  // https://snowflake.slack.com/archives/C09TH5U3QP2/p1779268894662169
  it.todo('returns FLOAT special values "NaN", "inf", "-inf" as JS types', async () => {
    const { rows } = await executeAsync(
      connection,
      `SELECT
        'NaN'::FLOAT as NAN_COLUMN,
        'inf'::FLOAT as INF_COLUMN,
        '-inf'::FLOAT as -INF_COLUMN
    `,
    );
    expect(rows![0].NAN_COLUMN).toBe(NaN);
    expect(rows![0].INF_COLUMN).toBe(Infinity);
    expect(rows![0]['-INF_COLUMN']).toBe(-Infinity);
  });

  it.each([
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
  ])('returns integer as BigInt instance when $name is set', async ({ connectionFactory }) => {
    const bigIntConnection = await connectionFactory();
    try {
      const { statement, rows } = await executeAsync(
        bigIntConnection,
        'SELECT 90071992547409954434323 as INT_COLUMN',
      );
      const resultColumn = statement.getColumn(0);
      const selectedValue = rows![0].INT_COLUMN as typeof BigInteger;
      expect(resultColumn.getType()).toBe('fixed');
      expect(resultColumn.isNumber()).toBe(true);
      expect(BigInteger.isInstance(selectedValue)).toBe(true);
      expect(selectedValue.toString()).toBe('90071992547409954434323');
    } finally {
      await destroyConnectionAsync(bigIntConnection);
    }
  });

  it('returns DECFLOAT literals correctly formatted', async () => {
    const cases: { literal: string; expected: string | null }[] = [
      { literal: 'NULL', expected: null },
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
});
