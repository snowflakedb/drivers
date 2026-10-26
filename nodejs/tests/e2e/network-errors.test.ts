import { describe, expect, it, onTestFinished } from 'vitest';
import { createConnection } from './utils/fixtures.js';
import {
  connectAsyncWithErrorBD,
  findFreePort,
  IS_RUNNING_FOR_OLD_DRIVER,
  isRunningNewDriverWithBD,
} from './utils/index.js';
import { delayedLoginSuccess, WiremockServer } from './utils/wiremock/index.js';

describe('Network Errors', () => {
  it('should report NetworkError when Snowflake cannot be reached', async () => {
    const port = await findFreePort();
    const connection = createConnection({
      host: '127.0.0.1',
      port: String(port),
      protocol: 'http',
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

  it('should report NetworkError when login does not answer in time', async () => {
    const wiremock = await WiremockServer.spawn();
    onTestFinished(() => wiremock.destroy());
    await wiremock.stub(delayedLoginSuccess(20_000));
    const connection = createConnection({
      ...wiremock.connectionOptions,
      // 3.x has no deadline for the whole login; it gives up when an attempt times out and no retries remain.
      ...(IS_RUNNING_FOR_OLD_DRIVER
        ? { timeout: 1000, sfRetryMaxLoginRetries: 0 }
        : { retryTimeout: 2 }),
    });

    await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject({
      name: 'NetworkError',
      code: 401001,
      message: isRunningNewDriverWithBD('BD#79')
        ? 'Network error. Could not reach Snowflake.'
        : expect.stringMatching(/timeout of 1000ms exceeded/),
    });
  });
});
