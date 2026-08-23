import BigInteger from 'big-integer';
import { Connection } from 'snowflake-sdk';
import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import {
  createTestConnection,
  destroyConnectionAsync,
  executeAsync,
  getSnowflakeSDK,
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

  it('returns DECFLOAT as String', async () => {
    const { statement, rows } = await executeAsync(
      connection,
      "SELECT '-9.8765432099999998623226732747455716901e-250'::DECFLOAT as DECFLOAT_COLUMN",
    );
    // NO isDecfloat method available :/
    expect(statement.getColumn(0).getType()).toBe('decfloat');
    expect(rows![0].DECFLOAT_COLUMN).toBe('-9.8765432099999998623226732747455716901e-250');
  });
});
