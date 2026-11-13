import BigInteger from 'big-integer';
import { expect, onTestFinished } from 'vitest';
import type { Connection } from '../../types/sdk-types.js';
import { createConnection, createLiveConnection } from '../utils/fixtures.js';
import { IS_RUNNING_FOR_OLD_DRIVER, isRunningNewDriverWithBD } from '../utils/index.js';
import { getSessionParameterFromServer } from '../utils/query.js';

/**
 * The old driver wraps BigInt-mode values in a `big-integer` instance; the new driver returns a
 * native `bigint` (BD#8).
 */
export function isBigIntValue(value: unknown): boolean {
  return isRunningNewDriverWithBD('BD#8')
    ? typeof value === 'bigint'
    : BigInteger.isInstance(value);
}

export function expectBigIntValue(actual: unknown, expected: string): void {
  expect(isBigIntValue(actual)).toBe(true);
  expect(String(actual)).toBe(expected);
}

export function expectBigIntValues(actual: unknown[], expectedDigits: string[]): void {
  expect(actual).toHaveLength(expectedDigits.length);
  expectedDigits.forEach((expected, index) => {
    expectBigIntValue(actual[index], expected);
  });
}

/**
 * Returns a live connection that leaves NULL cells as `null` under `fetchAsString` instead of
 * rendering them as the string `'NULL'`.
 *
 * The old driver keeps `representNullAsStringNull` in module state, not on the connection, so the
 * `false` leaks into every later test in the process; the throwaway connection registered in
 * `onTestFinished` resets it. The new driver scopes the option to the connection (BD#22) and needs
 * no reset.
 */
export async function createLiveNullPreservingConnection(): Promise<Connection> {
  const connection = await createLiveConnection({ representNullAsStringNull: false });
  onTestFinished(() => {
    if (!isRunningNewDriverWithBD('BD#22')) {
      createConnection({ representNullAsStringNull: true });
    }
  });
  return connection;
}

/**
 * Returns true when query results on this connection come back as Arrow.
 *
 * The server sends Arrow only to drivers that support it, and the old (3.x) driver does not.
 * When the session is set to ARROW, the server reports ARROW but returns JSON to that driver.
 */
export async function isArrowResultFormat(connection: Connection): Promise<boolean> {
  if (IS_RUNNING_FOR_OLD_DRIVER) {
    return false;
  }
  return (await getSessionParameterFromServer(connection, 'JS_QUERY_RESULT_FORMAT')) === 'ARROW';
}
