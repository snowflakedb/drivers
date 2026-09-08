import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import type { Connection } from '../../types/sdk-types.js';
import {
  createTestConnection,
  destroyConnectionAsync,
  executeAsync,
  isRunningNewDriverWithBD,
} from '../utils/index.js';
import { getSessionParameterFromServer } from '../utils/query.js';

describe('Statement-level parameters', () => {
  const CUSTOM_TIME_FORMAT = 'HH24:MI:SS.FF9';
  const SELECT_TIME = "SELECT '12:34:56.789789789'::TIME::STRING AS TIME_VALUE";
  const FORMATTED_TIME = '12:34:56.789789789';
  const DEFAULT_TIME = '12:34:56';

  let connection: Connection;

  beforeAll(async () => {
    connection = createTestConnection();
    await connection.connectAsync();
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  it('should apply a parameter passed at the statement level', async () => {
    const { rows } = await executeAsync(connection, SELECT_TIME, {
      parameters: { TIME_OUTPUT_FORMAT: CUSTOM_TIME_FORMAT },
    });
    // The new driver drops a statement-level parameter, so the query runs under the
    // session DEFAULT_TIME; the old driver honors it for the one query (BD#27).
    if (isRunningNewDriverWithBD('BD#28')) {
      expect(rows[0].TIME_VALUE).toBe(DEFAULT_TIME);
    } else {
      expect(rows[0].TIME_VALUE).toBe(FORMATTED_TIME);
    }
  });

  it('should not modify the session when a parameter is passed at the statement level', async () => {
    const initialSessionValue = await getSessionParameterFromServer(
      connection,
      'TIME_OUTPUT_FORMAT',
    );
    await executeAsync(connection, SELECT_TIME, {
      parameters: { TIME_OUTPUT_FORMAT: CUSTOM_TIME_FORMAT },
    });
    const sessionValue = await getSessionParameterFromServer(connection, 'TIME_OUTPUT_FORMAT');
    expect(sessionValue).toBe(initialSessionValue);
  });
});
