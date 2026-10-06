import { describe, expect, it } from 'vitest';
import { createLiveConnection } from '../utils/fixtures.js';
import { getSessionParameterFromServer } from '../utils/query.js';

describe('tests/definitions/shared/session/session_parameters.feature', () => {
  it('should enable session keep-alive via connection string', async () => {
    // Given Snowflake client is logged in with connection option CLIENT_SESSION_KEEP_ALIVE set to "true"
    const connection = await createLiveConnection({ clientSessionKeepAlive: true });

    // When Query "SHOW PARAMETERS LIKE 'CLIENT_SESSION_KEEP_ALIVE'" is executed
    const value = await getSessionParameterFromServer(connection, 'CLIENT_SESSION_KEEP_ALIVE');

    // Then the session parameter value should be "true"
    expect(value).toBe('true');
  });

  it('should set heartbeat frequency via connection string', async () => {
    // Given Snowflake client is logged in with CLIENT_SESSION_KEEP_ALIVE=true and CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY=1800
    const connection = await createLiveConnection({
      clientSessionKeepAlive: true,
      clientSessionKeepAliveHeartbeatFrequency: 1800,
    });

    // When Query "SHOW PARAMETERS LIKE 'CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY'" is executed
    const value = await getSessionParameterFromServer(
      connection,
      'CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY',
    );

    // Then the session parameter value reflects the configured frequency
    expect(value).toBe('1800');
  });
});
