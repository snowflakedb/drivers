import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import {
  afterAll,
  afterEach,
  beforeAll,
  beforeEach,
  describe,
  expect,
  it,
  onTestFinished,
  vi,
} from 'vitest';
import type { Connection, ConnectionOptions } from '../../types/sdk-types.js';
import ErrorCode from '../../../src/constants/ErrorCode.js';
import { TempDir } from '../utils/files.js';
import {
  snowflake,
  baseConnectionOptions,
  connectAsyncWithErrorBD,
  destroyConnectionAsync,
  isRunningNewDriverWithBD,
  NOT_IMPLEMENTED_IN_NEW_DRIVER,
} from '../utils/index.js';
import { verifySimpleQuery } from '../utils/query.js';
import {
  WiremockServer,
  authenticatorRequestSuccess,
  logoutSuccess,
} from '../utils/wiremock/index.js';
import {
  expectConnectionNotUp,
  destroyConnectionAfterTest,
  NEVER_ESTABLISHED_QUERY,
  TERMINATED_QUERY,
  cleanBrowserProcesses,
  deliverBrowserCallbackAfter,
  NOT_IN_AUTH_TEST_CONTAINER,
  patchOldDriverHttpsProtocol,
  requireOktaCredentials,
  waitForChromium,
} from './utils.js';

const execFileAsync = promisify(execFile);

async function provideBrowserCredentials(
  mode: 'success' | 'fail',
  user: string,
  password: string,
): Promise<void> {
  await waitForChromium();
  await execFileAsync(
    'node',
    ['/externalbrowser/provideBrowserCredentials.js', mode, user, password],
    { timeout: 90_000 },
  );
}

function submitRejectedIdpCredentials(): Promise<void> {
  return provideBrowserCredentials('fail', 'itsnotanaccount.com', 'fakepassword');
}

async function connectWithBrowserAutomation(
  connect: () => Promise<unknown>,
  user: string,
  password: string,
): Promise<void> {
  const [connectionAttempt, provideCredentialsPromise] = await Promise.allSettled([
    connect(),
    provideBrowserCredentials('success', user, password),
  ]);
  onTestFinished(cleanBrowserProcesses);
  if (connectionAttempt.status === 'rejected') {
    throw connectionAttempt.reason;
  }
  if (provideCredentialsPromise.status === 'rejected') {
    throw provideCredentialsPromise.reason;
  }
}

function createExternalBrowserConnection(
  username: string,
  options: ConnectionOptions = {},
): Connection {
  return snowflake.createConnection({
    ...baseConnectionOptions,
    username,
    authenticator: 'EXTERNALBROWSER',
    clientStoreTemporaryCredential: false,
    ...options,
  });
}

function createWiremockExternalBrowserConnection(
  wiremock: WiremockServer,
  overrides: Partial<ConnectionOptions> = {},
): Connection {
  const wiremockOptions = wiremock.connectionOptions;
  return snowflake.createConnection({
    account: 'test_account',
    username: 'test_user',
    authenticator: 'EXTERNALBROWSER',
    // Old Driver getSSOURL ignores the separate `port` option, so we put it in `host`
    ...(isRunningNewDriverWithBD('BD#64')
      ? wiremockOptions
      : {
          // @ts-ignore BD#2 - protocol is not typed old driver ConnectionOptions
          protocol: wiremockOptions.protocol,
          // @ts-ignore BD#2 - port is not typed in old driver ConnectionOptions
          host: `${wiremockOptions.host}:${wiremockOptions.port}`,
        }),
    ...overrides,
  } satisfies ConnectionOptions);
}

describe('EXTERNALBROWSER authentication', () => {
  describe('tests/definitions/shared/authentication/external_browser.feature', () => {
    describe('WireMock external browser authentication', () => {
      let wiremock: WiremockServer;
      let previousHttpsProtocol: string | undefined;

      beforeAll(async () => {
        wiremock = await WiremockServer.spawn();
        if (!isRunningNewDriverWithBD('BD#64')) {
          previousHttpsProtocol = patchOldDriverHttpsProtocol('http');
        }
      });

      beforeEach(async () => {
        await wiremock.reset();
      });

      afterAll(async () => {
        if (previousHttpsProtocol !== undefined) {
          patchOldDriverHttpsProtocol(previousHttpsProtocol);
        }
        await wiremock.destroy();
      });

      it('should fail when authenticator-request returns forbidden', async () => {
        // Given Wiremock returns HTTP 403 for authenticator-request
        await wiremock.stubFromFile('auth/external_browser_authenticator_request_forbidden.json');

        // When Trying to Connect
        const connection = createWiremockExternalBrowserConnection(wiremock);

        // TODO(SNOW-3894966): this does not pin `sqlState` or `code`.
        //
        // Then Connection fails with authenticator error
        await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject({
          name: isRunningNewDriverWithBD('BD#65') ? 'Error' : 'AxiosError',
          message: isRunningNewDriverWithBD('BD#65')
            ? 'Failed to login: External browser SSO failed: HTTP 403 Forbidden from Snowflake during Snowflake authenticator-request: {"success":false,"message":"Forbidden"}'
            : 'Request failed with status code 403',
        });
      });

      it('should fail when authenticator-request returns logical failure', async () => {
        // Given Wiremock returns success false for authenticator-request
        await wiremock.stubFromFile(
          'auth/external_browser_authenticator_request_logical_failure.json',
        );

        // When Trying to Connect
        const connection = createWiremockExternalBrowserConnection(wiremock);

        // Then Connection fails with authenticator error
        await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject({
          name: 'Error',
          message: isRunningNewDriverWithBD('BD#67')
            ? 'Failed to login: External browser SSO failed: Snowflake authenticator-request (logical failure): External browser authentication is not enabled for this account, code: -1'
            : 'Authentication failed. Error code: undefined, message: External browser authentication is not enabled for this account',
          ...(isRunningNewDriverWithBD('BD#66') ? { code: undefined } : {}),
        });
      });

      describe('WireMock flows that would open a browser', () => {
        it('should login with external browser using simulated callback', async () => {
          // Given Wiremock returns valid ssoUrl and proofKey for authenticator-request
          await wiremock.stubFromFile('auth/external_browser_authenticator_request.json');

          // And Login endpoint returns success
          await wiremock.stubFromFile('auth/login_success_external_browser.json');
          await wiremock.stub(logoutSuccess());

          const token = 'browser_sso_token_12345';
          const connection = createWiremockExternalBrowserConnection(wiremock, {
            openExternalBrowserCallback: () => {
              deliverBrowserCallbackAfter(wiremock, token, 0);
            },
          });

          // When Trying to Connect with simulated browser callback delivering a token
          await connection.connectAsync();
          destroyConnectionAfterTest(connection);

          // Then Login is successful
          expect(connection.isUp()).toBe(true);

          // And Login request contains EXTERNALBROWSER authenticator, token, proof key, and login name
          const [loginRequest] = await wiremock.findRequests('/session/v1/login-request.*');
          const { data } = JSON.parse(loginRequest.body) as {
            data: {
              AUTHENTICATOR: string;
              TOKEN: string;
              PROOF_KEY: string;
              LOGIN_NAME: string;
            };
          };
          expect(data.AUTHENTICATOR).toBe('EXTERNALBROWSER');
          expect(data.TOKEN).toBe(token);
          expect(data.PROOF_KEY).toBe('mock_proof_key_abc123');
          expect(data.LOGIN_NAME).toBe('test_user');
        });

        it('should fail with timeout when no browser callback arrives', async () => {
          // Given Wiremock returns valid ssoUrl and proofKey for authenticator-request
          await wiremock.stubFromFile('auth/external_browser_authenticator_request.json');

          // And Authentication timeout is set to 2 seconds
          const connection = createWiremockExternalBrowserConnection(wiremock, {
            browserActionTimeout: 2000,
            openExternalBrowserCallback: () => {},
          });

          // When Trying to Connect without any browser callback
          const connectionAttempt = connectAsyncWithErrorBD(connection);

          // Then Connection fails with timeout or browser error
          await expect(connectionAttempt).rejects.toMatchObject({
            name: 'Error',
            message: isRunningNewDriverWithBD('BD#59')
              ? 'Failed to login: External browser SSO failed: Authentication timeout exceeded (budget 2s)'
              : 'Error while getting SAML token: Browser action timed out after 2000 ms.',
            ...(isRunningNewDriverWithBD('BD#66') ? { code: undefined } : {}),
          });
          // TODO(SNOW-3894966): this does not pin `sqlState`. The new driver sets `HYT00`; the old driver leaves it unset.
          await expectConnectionNotUp(
            connection,
            isRunningNewDriverWithBD('BD#61') ? TERMINATED_QUERY : NEVER_ESTABLISHED_QUERY,
          );
        });

        it('should fail when login request is rejected after browser callback', async () => {
          // Given Wiremock returns valid ssoUrl and proofKey for authenticator-request
          await wiremock.stubFromFile('auth/external_browser_authenticator_request.json');

          // And Login endpoint returns failure
          await wiremock.stubFromFile('auth/login_failure_external_browser.json');

          const token = 'browser_sso_token_rejected';
          const connection = createWiremockExternalBrowserConnection(wiremock, {
            openExternalBrowserCallback: () => {
              deliverBrowserCallbackAfter(wiremock, token, 0);
            },
          });

          // When Trying to Connect with simulated browser callback delivering a token
          const connectionAttempt = connectAsyncWithErrorBD(connection);

          // Then Connection fails with login error
          await expect(connectionAttempt).rejects.toMatchObject({
            // TODO(SNOW-3894966)
            name: NOT_IMPLEMENTED_IN_NEW_DRIVER ? 'Error' : 'OperationFailedError',
            message: isRunningNewDriverWithBD('BD#67')
              ? 'Failed to login: Login error: Invalid credentials, code: 390100'
              : 'Invalid credentials',
            code: '390100',
          });
          // TODO(SNOW-3894966): this does not pin `sqlState`. The new driver sets `28000`; the old driver leaves it unset.
          await expectConnectionNotUp(connection);
        });
      });

      it('should fail when authenticator-request reports SSO URL generation failure', async () => {
        // Given Wiremock returns SSO URL generation failure for authenticator-request
        await wiremock.stubFromFile(
          'auth/external_browser_authenticator_request_sso_url_error.json',
        );

        // When Trying to Connect
        const connection = createWiremockExternalBrowserConnection(wiremock);

        // Authenticator-request rejections put `390511` in the message rather than `error.code`.
        // TODO(SNOW-3894966): attach it as `code` the same way as login-request rejections.
        //
        // Then Connection fails with error 390511
        await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject({
          name: 'Error',
          message: isRunningNewDriverWithBD('BD#67')
            ? "Failed to login: External browser SSO failed: Snowflake authenticator-request (logical failure): SSO URL generation failed in External browser's SAML Request flow, code: 390511"
            : "Authentication failed. Error code: 390511, message: SSO URL generation failed in External browser's SAML Request flow",
          ...(isRunningNewDriverWithBD('BD#66') ? { code: undefined } : {}),
        });
        expect(connection.isUp()).toBe(false);
      });
    });

    describe.skipIf(NOT_IN_AUTH_TEST_CONTAINER)('Live external browser authentication', () => {
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
        destroyConnectionAfterTest(connection);

        // Then Login is successful and simple query can be executed
        await verifySimpleQuery(connection);
      });

      it('should reuse cached ID token without browser interaction', async () => {
        // Given External browser authentication is configured with caching enabled and a token has been cached from a previous connection
        const { user, password } = requireOktaCredentials();
        const browserLoginConnection = createExternalBrowserConnection(user, {
          clientStoreTemporaryCredential: true,
        });
        await connectWithBrowserAutomation(
          () => browserLoginConnection.connectAsync(),
          user,
          password,
        );
        destroyConnectionAfterTest(browserLoginConnection);
        await verifySimpleQuery(browserLoginConnection);

        // When Trying to Connect without browser interaction
        const cachedTokenConnection = createExternalBrowserConnection(user, {
          clientStoreTemporaryCredential: true,
          browserActionTimeout: 2000,
          openExternalBrowserCallback: () => {
            throw new Error('Browser should not be launched');
          },
        });
        await cachedTokenConnection.connectAsync();
        destroyConnectionAfterTest(cachedTokenConnection);

        // Then Login is successful and simple query can be executed
        await verifySimpleQuery(cachedTokenConnection);
      });
    });
  });

  describe('WireMock EXTERNALBROWSER behavior', () => {
    let wiremock: WiremockServer;
    let previousHttpsProtocol: string | undefined;

    beforeAll(async () => {
      wiremock = await WiremockServer.spawn();
      if (!isRunningNewDriverWithBD('BD#64')) {
        previousHttpsProtocol = patchOldDriverHttpsProtocol('http');
      }
    });

    beforeEach(async () => {
      await wiremock.reset();
    });

    afterAll(async () => {
      if (previousHttpsProtocol !== undefined) {
        patchOldDriverHttpsProtocol(previousHttpsProtocol);
      }
      await wiremock.destroy();
    });

    // The old driver starts browserActionTimeout when it hands the SSO URL to the browser; the
    // new driver starts it before the authenticator-request, so a slow SSO round trip spends
    // most of the budget before the browser is even open (BD#59). The callback below lands
    // inside the old driver's untouched window and past the 500 ms the new driver has left.
    it('should charge the authenticator-request round trip to the browser budget', async () => {
      const authenticatorRequest = authenticatorRequestSuccess();
      await wiremock.stub({
        ...authenticatorRequest,
        response: { ...authenticatorRequest.response, fixedDelayMilliseconds: 3500 },
      });
      await wiremock.stubFromFile('auth/login_success_external_browser.json');
      await wiremock.stub(logoutSuccess());

      let callbackDelivery = Promise.resolve();
      const connection = createWiremockExternalBrowserConnection(wiremock, {
        browserActionTimeout: 4000,
        openExternalBrowserCallback: () => {
          callbackDelivery = deliverBrowserCallbackAfter(
            wiremock,
            'browser_sso_token_12345',
            2000,
          ).catch(() => {});
        },
      });

      const connectionAttempt = connection.connectAsync();
      if (isRunningNewDriverWithBD('BD#59')) {
        await expect(connectionAttempt).rejects.toMatchObject({
          name: 'Error',
          message:
            'Failed to login: External browser SSO failed: Authentication timeout exceeded (budget 4s)',
          ...(isRunningNewDriverWithBD('BD#66') ? { code: undefined } : {}),
        });
        // Connect rejects at the 4s budget before this delayed callback is written.
        // Wait for it so the pending write does not run during the next test.
        await callbackDelivery;
      } else {
        await connectionAttempt;
        destroyConnectionAfterTest(connection);
        expect(connection.isUp()).toBe(true);
      }
    });

    it('should refuse to destroy a connection after a no-callback timeout', async () => {
      await wiremock.stubFromFile('auth/external_browser_authenticator_request.json');
      const connection = createWiremockExternalBrowserConnection(wiremock, {
        browserActionTimeout: 2000,
        openExternalBrowserCallback: () => {},
      });
      await expect(connectAsyncWithErrorBD(connection)).rejects.toBeInstanceOf(Error);
      await expect(destroyConnectionAsync(connection)).rejects.toMatchObject(
        isRunningNewDriverWithBD('BD#62')
          ? {
              name: 'ClientError',
              code: ErrorCode.ERR_CONN_DESTROY_STATUS_DISCONNECTED,
              message: 'Already disconnected.',
            }
          : {
              name: 'ClientError',
              code: ErrorCode.ERR_CONN_DESTROY_STATUS_PRISTINE,
              message: 'Not connected, so nothing to destroy.',
            },
      );
    });

    it('should fail when login request reports an IdP username mismatch', async () => {
      await wiremock.stubFromFile('auth/external_browser_authenticator_request.json');
      await wiremock.stubFromFile('auth/login_failure_external_browser_usernames_mismatch.json');

      const token = 'browser_sso_token_mismatched_user';
      const connection = createWiremockExternalBrowserConnection(wiremock, {
        openExternalBrowserCallback: () => {
          deliverBrowserCallbackAfter(wiremock, token, 0);
        },
      });
      const connectionAttempt = connectAsyncWithErrorBD(connection);

      await expect(connectionAttempt).rejects.toMatchObject({
        // TODO(SNOW-3894966)
        name: NOT_IMPLEMENTED_IN_NEW_DRIVER ? 'Error' : 'OperationFailedError',
        message: isRunningNewDriverWithBD('BD#67')
          ? 'Failed to login: Login error: The user you were trying to authenticate as differs from the user currently logged in at the IDP., code: 390191'
          : 'The user you were trying to authenticate as differs from the user currently logged in at the IDP.',
        code: '390191',
      });
      await expectConnectionNotUp(connection);
    });
  });

  describe.skipIf(NOT_IN_AUTH_TEST_CONTAINER)('Live external browser timeout', () => {
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

    it('should fail with timeout when the IdP rejects the credentials', async () => {
      const { user } = requireOktaCredentials();
      const connection = createExternalBrowserConnection(user, {
        browserActionTimeout: 2000,
      });

      const [connectionAttempt, provideCredentialsPromise] = await Promise.allSettled([
        connectAsyncWithErrorBD(connection),
        submitRejectedIdpCredentials(),
      ]);

      expect(provideCredentialsPromise.status).toBe('fulfilled');
      expect(connectionAttempt.status).toBe('rejected');
      if (connectionAttempt.status !== 'rejected') {
        return;
      }
      expect(connectionAttempt.reason).toMatchObject({
        name: 'Error',
        message: isRunningNewDriverWithBD('BD#59')
          ? 'Failed to login: External browser SSO failed: Authentication timeout exceeded (budget 2s)'
          : 'Error while getting SAML token: Browser action timed out after 2000 ms.',
        ...(isRunningNewDriverWithBD('BD#66') ? { code: undefined } : {}),
      });
      await expectConnectionNotUp(
        connection,
        isRunningNewDriverWithBD('BD#61') ? TERMINATED_QUERY : NEVER_ESTABLISHED_QUERY,
      );
    });
  });

  it('should reject disableConsoleLogin as unknown on the new driver and accept it on the old driver', () => {
    const options = {
      account: 'test_account',
      username: 'test_user',
      authenticator: 'EXTERNALBROWSER',
      disableConsoleLogin: true,
    } satisfies ConnectionOptions;
    if (isRunningNewDriverWithBD('BD#63')) {
      expect(() => snowflake.createConnection(options)).toThrow(
        'Unknown connection option: disableConsoleLogin',
      );
    } else {
      expect(() => snowflake.createConnection(options)).not.toThrow();
    }
  });
});
