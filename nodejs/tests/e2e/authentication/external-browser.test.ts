import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { afterEach, beforeEach, describe, expect, it, onTestFinished, vi } from 'vitest';
import type { Connection, ConnectionOptions } from '../../types/sdk-types.js';
import { TempDir } from '../utils/files.js';
import {
  snowflake,
  baseConnectionOptions,
  destroyConnectionAsync,
  executeAsync,
  NOT_IMPLEMENTED_IN_NEW_DRIVER,
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
  onTestFinished(cleanBrowserProcesses);
  if (connectResult.status === 'rejected') {
    throw connectResult.reason;
  }
  if (browserResult.status === 'rejected') {
    throw browserResult.reason;
  }
}

function createExternalBrowserConnection(
  username: string,
  options: ConnectionOptions = {},
): Connection {
  const connection = snowflake.createConnection({
    ...baseConnectionOptions,
    username,
    authenticator: 'EXTERNALBROWSER',
    clientStoreTemporaryCredential: false,
    ...options,
  });
  onTestFinished(() => destroyConnectionAsync(connection));
  return connection;
}

async function verifySimpleQuery(connection: Connection): Promise<void> {
  const { rows } = await executeAsync(connection, 'SELECT 1');
  expect(rows).toHaveLength(1);
  expect(Object.values(rows[0])).toEqual([1]);
}

describe.skipIf(NOT_IN_AUTH_TEST_CONTAINER)(
  'EXTERNALBROWSER authentication (tests/definitions/shared/authentication/external_browser.feature)',
  () => {
    let tokenCacheDir: TempDir;

    beforeEach(() => {
      tokenCacheDir = new TempDir();
      vi.stubEnv('SF_TEMPORARY_CREDENTIAL_CACHE_DIR', tokenCacheDir.path);
    });

    afterEach(async () => {
      vi.unstubAllEnvs();
      tokenCacheDir.cleanup();
      await cleanBrowserProcesses();
    });

    it('should authenticate with external browser via Okta IdP', async () => {
      // Given External browser authentication is configured with valid Okta user
      const { user, password } = requireOktaCredentials();
      const connection = createExternalBrowserConnection(user);

      // When Trying to Connect with headless browser providing valid credentials
      await connectWithBrowserAutomation(() => connection.connectAsync(), user, password);

      // Then Login is successful and simple query can be executed
      await verifySimpleQuery(connection);
    });

    // TODO: needs openExternalBrowserCallback to be implemented
    it.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)(
      'should reuse cached ID token without browser interaction',
      async () => {
        // Given External browser authentication is configured with caching enabled and a token has been cached from a previous connection
        const { user, password } = requireOktaCredentials();
        const first = createExternalBrowserConnection(user, {
          clientStoreTemporaryCredential: true,
        });
        await connectWithBrowserAutomation(() => first.connectAsync(), user, password);
        await verifySimpleQuery(first);

        // When Trying to Connect without browser interaction
        const second = createExternalBrowserConnection(user, {
          clientStoreTemporaryCredential: true,
          browserActionTimeout: 2000,
          openExternalBrowserCallback: () => {
            throw new Error('Browser should not be launched');
          },
        });
        await second.connectAsync();

        // Then Login is successful and simple query can be executed
        await verifySimpleQuery(second);
      },
    );
  },
);
