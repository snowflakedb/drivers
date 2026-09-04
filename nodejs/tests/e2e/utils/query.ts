import type { Connection } from '../../types/sdk-types.js';
import { executeAsync } from './index.js';

export async function getSessionParameterFromServer(
  connection: Connection,
  name: string,
): Promise<unknown> {
  const { rows } = await executeAsync(connection, `SHOW PARAMETERS LIKE '${name}'`);
  if (rows.length !== 1) {
    throw new Error(`SHOW PARAMETERS LIKE '${name}' returned ${rows.length} rows, expected 1`);
  }
  return rows[0].value;
}

export async function setSessionParameter(
  connection: Connection,
  name: string,
  value: boolean | number | string,
): Promise<void> {
  await executeAsync(connection, `ALTER SESSION SET ${name} = ${value}`);
}
