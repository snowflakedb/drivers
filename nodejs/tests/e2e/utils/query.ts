import type { Connection } from '../../types/sdk-types.js';
import { executeAsync, randomizeName } from './index.js';

export async function withTemporaryTable<T>(
  connection: Connection,
  columns: string,
  body: (tableName: string) => Promise<T>,
): Promise<T> {
  const tableName = randomizeName('nodejs_');
  await executeAsync(connection, `CREATE OR REPLACE TEMPORARY TABLE ${tableName} (${columns})`);
  try {
    return await body(tableName);
  } finally {
    await executeAsync(connection, `DROP TABLE IF EXISTS ${tableName}`);
  }
}

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

export async function unsetSessionParameter(connection: Connection, name: string): Promise<void> {
  await executeAsync(connection, `ALTER SESSION UNSET ${name}`);
}
