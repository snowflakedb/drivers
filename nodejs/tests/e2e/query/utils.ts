import type { Connection } from '../../types/sdk-types.js';
import { createTestConnection, destroyConnectionAsync, getSnowflakeSDK } from '../utils/index.js';

/**
 * Runs `useConnection` against a connection that leaves NULL cells as `null` under
 * `fetchAsString` instead of rendering them as the string `'NULL'`.
 *
 * The old driver keeps `representNullAsStringNull` in module state written by
 * `createConnection`, not on the connection object, so the option outlives the connection that
 * set it and every later test in the process would inherit `false`. Restoring it belongs here
 * rather than in each caller's `finally`.
 */
export async function withNullPreservingConnection(
  snowflake: ReturnType<typeof getSnowflakeSDK>,
  useConnection: (connection: Connection) => Promise<void>,
): Promise<void> {
  const connection = createTestConnection(snowflake, { representNullAsStringNull: false });
  try {
    await connection.connectAsync();
    await useConnection(connection);
  } finally {
    createTestConnection(snowflake, { representNullAsStringNull: true });
    await destroyConnectionAsync(connection);
  }
}
