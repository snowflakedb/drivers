import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { afterEach, beforeEach, describe, expect, it, onTestFinished } from 'vitest';
import {
  baseConnectionOptions,
  destroyConnectionAsync,
  executeAsync,
  snowflake,
} from '../utils/index.js';
import {
  cleanBrowserProcesses,
  NOT_IN_AUTH_TEST_CONTAINER,
  requireOktaCredentials,
  waitForChromium,
} from './utils.js';

const execFileAsync = promisify(execFile);

async function provideBrowserCredentials(user: string, password: string): Promise<void> {
  await waitForChromium();
  await execFileAsync(
    'node',
    ['/externalbrowser/provideBrowserCredentials.js', 'success', user, password],
    { timeout: 90_000 },
  );
}

async function connectWithBrowserAutomation(
  connect: () => Promise<unknown>,
  user: string,
  password: string,
): Promise<void> {
  const [connectResult, browserResult] = await Promise.allSettled([
    connect(),
    provideBrowserCredentials(user, password),
  ]);
  if (connectResult.status === 'rejected') {
    throw connectResult.reason;
  }
  if (browserResult.status === 'rejected') {
    throw browserResult.reason;
  }
}

describe.skipIf(NOT_IN_AUTH_TEST_CONTAINER)('External browser authentication', () => {
  beforeEach(async () => {
    await cleanBrowserProcesses();
  });

  afterEach(async () => {
    await cleanBrowserProcesses();
  });

  it('should authenticate with external browser via Okta IdP', async () => {
    // Given External browser authentication is configured with valid Okta user
    const { user, password } = requireOktaCredentials();
    const connection = snowflake.createConnection({
      ...baseConnectionOptions,
      username: user,
      authenticator: 'EXTERNALBROWSER',
    });
    onTestFinished(async () => {
      await destroyConnectionAsync(connection);
    });

    // When Trying to Connect with headless browser providing valid credentials
    await connectWithBrowserAutomation(() => connection.connectAsync(), user, password);

    // Then Login is successful and simple query can be executed
    const { rows } = await executeAsync(connection, 'SELECT 1');
    expect(rows).toHaveLength(1);
    expect(Object.values(rows[0])).toEqual([1]);
  });
});
