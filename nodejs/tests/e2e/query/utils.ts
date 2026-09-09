import { onTestFinished } from 'vitest';
import type { Connection } from '../../types/sdk-types.js';
import { createConnection, createLiveConnection } from '../utils/fixtures.js';
import { isRunningNewDriverWithBD } from '../utils/index.js';

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
