import BigInteger from 'big-integer';
import { describe, it, expect, afterEach } from 'vitest';
import type { Connection, RowStatement } from '../types/sdk-types.js';
import {
  collectStreamedRows,
  createTestConnection,
  destroyConnectionAsync,
  executeAsync,
  getSnowflakeSDK,
  isRunningNewDriverWithBD,
} from './utils/index.js';
import { getSessionParameterFromServer, setSessionParameter } from './utils/query.js';

// How session parameters travel: out to the server when set as a connection option,
// and back in to drive row decoding, including after an ALTER SESSION.
describe('JS_TREAT_INTEGER_AS_BIGINT', () => {
  const PARAMETER_NAME = 'JS_TREAT_INTEGER_AS_BIGINT';
  const snowflake = getSnowflakeSDK();
  let connection: Connection | undefined;

  afterEach(async () => {
    if (connection !== undefined) {
      await destroyConnectionAsync(connection);
      connection = undefined;
    }
  });

  function expectBigInt(value: unknown, expected: string): void {
    // Old driver wraps BigInt-mode values in a `big-integer` instance; the new driver
    // returns a native `bigint` (BD#8).
    const isBigInt = isRunningNewDriverWithBD('BD#8')
      ? typeof value === 'bigint'
      : BigInteger.isInstance(value);
    expect(isBigInt).toBe(true);
    expect(String(value)).toBe(expected);
  }

  async function selectInteger(
    connectionToQuery: Connection,
    options: { streamResult?: boolean } = {},
  ): Promise<unknown> {
    const { statement, rows } = await executeAsync(
      connectionToQuery,
      'SELECT 7::INT AS INT_COLUMN',
      options,
    );
    if (options.streamResult !== true) {
      return rows[0].INT_COLUMN;
    }
    const streamedRows = await collectStreamedRows(statement as RowStatement);
    expect(streamedRows).toHaveLength(1);
    return streamedRows[0].INT_COLUMN;
  }

  it('sets the session parameter on the server when passed as a connection option', async () => {
    connection = createTestConnection(snowflake, { jsTreatIntegerAsBigInt: true });
    await connection.connectAsync();

    expect(await getSessionParameterFromServer(connection, PARAMETER_NAME)).toBe('true');
  });

  it('stays at the server default when the connection option is absent', async () => {
    connection = createTestConnection(snowflake);
    await connection.connectAsync();

    expect(await getSessionParameterFromServer(connection, PARAMETER_NAME)).toBe('false');
    expect(await selectInteger(connection)).toBe(7);
  });

  it('sets the session parameter to false on the server when the connection option is false', async () => {
    connection = createTestConnection(snowflake, { jsTreatIntegerAsBigInt: false });
    await connection.connectAsync();

    expect(await getSessionParameterFromServer(connection, PARAMETER_NAME)).toBe('false');
    expect(await selectInteger(connection)).toBe(7);
  });

  it('maps integers as BigInt when set as a connection option', async () => {
    connection = createTestConnection(snowflake, { jsTreatIntegerAsBigInt: true });
    await connection.connectAsync();

    expectBigInt(await selectInteger(connection), '7');
  });

  it('maps streamed integers as BigInt when set as a connection option', async () => {
    connection = createTestConnection(snowflake, { jsTreatIntegerAsBigInt: true });
    await connection.connectAsync();

    expectBigInt(await selectInteger(connection, { streamResult: true }), '7');
  });

  it('takes effect on an already connected session via ALTER SESSION', async () => {
    connection = createTestConnection(snowflake);
    await connection.connectAsync();
    expect(await selectInteger(connection)).toBe(7);

    await setSessionParameter(connection, PARAMETER_NAME, true);

    expect(await getSessionParameterFromServer(connection, PARAMETER_NAME)).toBe('true');
    expectBigInt(await selectInteger(connection), '7');
  });

  it('stops applying when ALTER SESSION turns off what the connection option enabled', async () => {
    connection = createTestConnection(snowflake, { jsTreatIntegerAsBigInt: true });
    await connection.connectAsync();
    expectBigInt(await selectInteger(connection), '7');

    await setSessionParameter(connection, PARAMETER_NAME, false);

    expect(await getSessionParameterFromServer(connection, PARAMETER_NAME)).toBe('false');
    expect(await selectInteger(connection)).toBe(7);
  });

  it('applies per connection, not per process', async () => {
    connection = createTestConnection(snowflake, { jsTreatIntegerAsBigInt: true });
    await connection.connectAsync();
    const defaultConnection = createTestConnection(snowflake);
    await defaultConnection.connectAsync();

    try {
      expectBigInt(await selectInteger(connection), '7');
      expect(await selectInteger(defaultConnection)).toBe(7);
    } finally {
      await destroyConnectionAsync(defaultConnection);
    }
  });

  it('decodes a streamed result with the setting current when its rows are first read', async () => {
    connection = createTestConnection(snowflake);
    await connection.connectAsync();
    const { statement } = await executeAsync(connection, 'SELECT 7::INT AS INT_COLUMN', {
      streamResult: true,
    });

    await setSessionParameter(connection, PARAMETER_NAME, true);
    const rows = await collectStreamedRows(statement as RowStatement);

    expect(rows).toHaveLength(1);
    if (isRunningNewDriverWithBD('BD#21')) {
      expectBigInt(rows[0].INT_COLUMN, '7');
    } else {
      expect(rows[0].INT_COLUMN).toBe(7);
    }
  });
});
