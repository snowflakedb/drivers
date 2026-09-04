import type { Connection } from '../../types/sdk-types.js';
import {
  createTestConnection,
  destroyConnectionAsync,
  getSnowflakeSDK,
  isRunningNewDriverWithBD,
} from '../utils/index.js';

/**
 * Runs `useConnection` against a connection that leaves NULL cells as `null` under
 * `fetchAsString` instead of rendering them as the string `'NULL'`.
 *
 * The old driver keeps `representNullAsStringNull` in module state, not on the connection, so
 * the `false` leaks into every later test in the process; the throwaway connection in `finally`
 * resets it. The new driver scopes the option to the connection (BD#22) and needs no reset.
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
    if (!isRunningNewDriverWithBD('BD#22')) {
      createTestConnection(snowflake, { representNullAsStringNull: true });
    }
    await destroyConnectionAsync(connection);
  }
}
