import { describe, expect, it } from 'vitest';
import { createConnection } from './utils/fixtures.js';
import {
  connectAsyncWithErrorBD,
  findFreePort,
  IS_RUNNING_FOR_OLD_DRIVER,
  isRunningNewDriverWithBD,
} from './utils/index.js';

describe('Network Errors', () => {
  it('should report NetworkError when Snowflake cannot be reached', async () => {
    const port = await findFreePort();
    const connection = createConnection({
      host: '127.0.0.1',
      port: String(port),
      protocol: 'http',
      useEnvProxy: false,
      // 3.x retries a refused login 7 times with backoff by default; the new driver rejects this option as unknown.
      ...(IS_RUNNING_FOR_OLD_DRIVER ? { sfRetryMaxLoginRetries: 0 } : {}),
    });

    await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject({
      name: 'NetworkError',
      code: 401001,
      message: isRunningNewDriverWithBD('BD#79')
        ? 'Network error. Could not reach Snowflake.'
        : expect.stringMatching(/ECONNREFUSED/),
    });
  });
});
