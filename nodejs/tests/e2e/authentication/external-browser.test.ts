import { once } from 'node:events';
import { createServer, type AddressInfo } from 'node:net';
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
  createCallbackPage,
  cleanBrowserProcesses,
  connectWithBrowserAutomation,
  deliverBrowserCallbackAfter,
  deliverBrowserRedirectAfter,
  getHttpBody,
  patchOldDriverHttpsProtocol,
  provideBrowserCredentials,
  requireOktaCredentials,
} from './utils.js';

function submitRejectedIdpCredentials(): Promise<void> {
  return provideBrowserCredentials('fail', 'itsnotanaccount.com', 'fakepassword');
}

function connectWithExternalBrowser(
  connect: () => Promise<unknown>,
  user: string,
  password: string,
): Promise<void> {
  return connectWithBrowserAutomation(connect, () =>
    provideBrowserCredentials('success', user, password),
  );
}

async function findAvailablePort(): Promise<number> {
  const server = createServer().listen(0, '127.0.0.1');
  await once(server, 'listening');
  const { port } = server.address() as AddressInfo;
  server.close();
  return port;
}

// Old driver listens on `::`. New driver listens on 127.0.0.1.
// On macOS those do not conflict, so both are held. On Linux and Windows `::`
// is dual-stack and already owns 127.0.0.1, so the second listen fails with EADDRINUSE.
// `exclusive` keeps SO_REUSEADDR off. macOS otherwise lets the new driver share the port.
async function createOccupiedPort(): Promise<number> {
  const anyInterface = createServer();
  anyInterface.listen({ port: 0, exclusive: true });
  await once(anyInterface, 'listening');
  const { port } = anyInterface.address() as AddressInfo;
  const loopback = createServer();
  // `events.once(listening)` rejects on `error`, so it cannot race the EADDRINUSE case.
  const heldLoopback = await new Promise<boolean>((resolve, reject) => {
    loopback.once('listening', () => resolve(true));
    loopback.once('error', (error: NodeJS.ErrnoException) => {
      if (error.code === 'EADDRINUSE') {
        resolve(false);
        return;
      }
      reject(error);
    });
    loopback.listen({ port, host: '127.0.0.1', exclusive: true });
  });
  onTestFinished(() => {
    anyInterface.close();
    if (heldLoopback) {
      loopback.close();
    }
  });
  return port;
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
      let restoreHttpsProtocol: (() => void) | undefined;

      beforeAll(async () => {
        wiremock = await WiremockServer.spawn();
        if (!isRunningNewDriverWithBD('BD#64')) {
          restoreHttpsProtocol = patchOldDriverHttpsProtocol('http');
        }
      });

      beforeEach(async () => {
        await wiremock.reset();
      });

      afterAll(async () => {
        restoreHttpsProtocol?.();
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

        it('should login with the callback listener on browserRedirectPort', async () => {
          // Given A free local port
          const port = await findAvailablePort();

          // And Wiremock returns valid ssoUrl and proofKey, and login succeeds
          await wiremock.stubFromFile('auth/external_browser_authenticator_request.json');
          await wiremock.stubFromFile('auth/login_success_external_browser.json');
          await wiremock.stub(logoutSuccess());

          const connection = createWiremockExternalBrowserConnection(wiremock, {
            browserRedirectPort: port,
            openExternalBrowserCallback: () => {
              deliverBrowserCallbackAfter(wiremock, 'browser_sso_token_12345', 0);
            },
          });

          // When Trying to Connect
          await connection.connectAsync();
          destroyConnectionAfterTest(connection);

          // Then Login is successful
          expect(connection.isUp()).toBe(true);

          // And authenticator-request sends the configured port
          const [authenticatorRequest] = await wiremock.findRequests(
            '/session/authenticator-request.*',
          );
          const { data } = JSON.parse(authenticatorRequest.body) as {
            data: { BROWSER_MODE_REDIRECT_PORT: string };
          };
          expect(data.BROWSER_MODE_REDIRECT_PORT).toBe(String(port));
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
            name: 'OperationFailedError',
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

    describe(
      'Live external browser authentication',
      { tags: ['requires_auth_test_container'] },
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
          await connectWithExternalBrowser(() => connection.connectAsync(), user, password);
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
          await connectWithExternalBrowser(
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
      },
    );
  });

  describe('WireMock EXTERNALBROWSER behavior', () => {
    let wiremock: WiremockServer;
    let restoreHttpsProtocol: (() => void) | undefined;

    beforeAll(async () => {
      wiremock = await WiremockServer.spawn();
      if (!isRunningNewDriverWithBD('BD#64')) {
        restoreHttpsProtocol = patchOldDriverHttpsProtocol('http');
      }
    });

    beforeEach(async () => {
      await wiremock.reset();
    });

    afterAll(async () => {
      restoreHttpsProtocol?.();
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
          ).then(
            () => undefined,
            () => undefined,
          );
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
        name: 'OperationFailedError',
        message: isRunningNewDriverWithBD('BD#67')
          ? 'Failed to login: Login error: The user you were trying to authenticate as differs from the user currently logged in at the IDP., code: 390191'
          : 'The user you were trying to authenticate as differs from the user currently logged in at the IDP.',
        code: '390191',
      });
      await expectConnectionNotUp(connection);
    });

    it('should log in with a custom browserResponseRenderer', async () => {
      await wiremock.stubFromFile('auth/external_browser_authenticator_request.json');
      await wiremock.stubFromFile('auth/login_success_external_browser.json');
      await wiremock.stub(logoutSuccess());

      const { page, capture } = createCallbackPage();
      const connection = createWiremockExternalBrowserConnection(wiremock, {
        browserResponseRenderer: () => 'BROWSER OK',
        openExternalBrowserCallback: () => {
          capture(deliverBrowserCallbackAfter(wiremock, 'browser_sso_token_12345', 0));
        },
      });

      await Promise.all([connection.connectAsync(), page]);
      destroyConnectionAfterTest(connection);

      expect(connection.isUp()).toBe(true);
      await expect(page.then(getHttpBody)).resolves.toBe('BROWSER OK');
    });

    it('should fail connect when browserResponseRenderer throws', async () => {
      await wiremock.stubFromFile('auth/external_browser_authenticator_request.json');
      await wiremock.stub(logoutSuccess());

      const renderedPage = isRunningNewDriverWithBD('BD#172') ? createCallbackPage() : undefined;
      const connection = createWiremockExternalBrowserConnection(wiremock, {
        browserResponseRenderer: () => {
          throw new Error('render failed');
        },
        openExternalBrowserCallback: () => {
          const delivery = deliverBrowserCallbackAfter(wiremock, 'browser_sso_token_12345', 0);
          renderedPage?.capture(delivery);
        },
      });

      await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject({
        message: isRunningNewDriverWithBD('BD#67')
          ? expect.stringMatching(
              /^Failed to login: External browser SSO failed: Browser response renderer failed: Error: render failed\n/,
            )
          : expect.stringContaining('render failed'),
      });
      if (renderedPage) {
        await expect(renderedPage.page).resolves.toBe(
          'HTTP/1.1 500 Internal Server Error\r\n' +
            'Content-Length: 0\r\n' +
            'Connection: close\r\n\r\n',
        );
      }
      await expectConnectionNotUp(
        connection,
        isRunningNewDriverWithBD('BD#61') ? TERMINATED_QUERY : NEVER_ESTABLISHED_QUERY,
      );
    });

    it('should serve the default page when browserResponseRenderer is not set', async () => {
      await wiremock.stubFromFile('auth/external_browser_authenticator_request.json');
      await wiremock.stubFromFile('auth/login_success_external_browser.json');
      await wiremock.stub(logoutSuccess());

      const { page, capture } = createCallbackPage();
      const connection = createWiremockExternalBrowserConnection(wiremock, {
        openExternalBrowserCallback: () => {
          capture(deliverBrowserCallbackAfter(wiremock, 'browser_sso_token_12345', 0));
        },
      });

      await connection.connectAsync();
      destroyConnectionAfterTest(connection);

      expect(connection.isUp()).toBe(true);
      await expect(page).resolves.toBe(
        isRunningNewDriverWithBD('BD#171')
          ? 'HTTP/1.1 200 OK\r\n' +
              'Content-Type: text/html; charset=utf-8\r\n' +
              'Content-Length: 168\r\n' +
              'Connection: close\r\n\r\n' +
              '<!DOCTYPE html><html><head><title>Authentication Successful</title></head>' +
              '<body><h1>Your identity was confirmed</h1><p>You can close this browser tab.</p>' +
              '</body></html>'
          : 'HTTP/1.1 200 OK\r\n' +
              'Content-Type: text/html; charset=utf-8\r\n' +
              'Connection: close\r\n\r\n' +
              'Your identity was confirmed and propagated to Snowflake Node.js driver. ' +
              'You can close this window now and go back where you started from.',
      );
    });

    it('should fail connect on an identity provider error callback', async () => {
      await wiremock.stubFromFile('auth/external_browser_authenticator_request.json');

      const { page, capture } = createCallbackPage();
      const connection = createWiremockExternalBrowserConnection(wiremock, {
        browserActionTimeout: 2000,
        openExternalBrowserCallback: () => {
          capture(
            deliverBrowserRedirectAfter(
              wiremock,
              { error: 'access_denied', error_description: 'denied' },
              0,
            ),
          );
        },
      });

      const connectionAttempt = connectAsyncWithErrorBD(connection);
      if (isRunningNewDriverWithBD('BD#169')) {
        await expect(connectionAttempt).rejects.toMatchObject({
          name: 'Error',
          message:
            'Failed to login: External browser SSO failed: Authentication timeout exceeded (budget 2s)',
          code: undefined,
        });
        await expect(page).resolves.toBe('');
      } else {
        await expect(connectionAttempt).rejects.toMatchObject({
          name: 'Error',
          message:
            'Error while getting SAML token: Error while getting oauth authorization code. ErrorCode access_denied. Message: denied',
        });
        await expect(page.then(getHttpBody)).resolves.toBe(
          'Error while getting oauth authorization code. ErrorCode access_denied. Message: denied',
        );
      }
      await expectConnectionNotUp(
        connection,
        isRunningNewDriverWithBD('BD#61') ? TERMINATED_QUERY : NEVER_ESTABLISHED_QUERY,
      );
    });
  });

  describe('Live external browser timeout', { tags: ['requires_auth_test_container'] }, () => {
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

  it('should report the occupied browserRedirectPort', async () => {
    const port = await createOccupiedPort();
    const connection = snowflake.createConnection({
      account: 'test_account',
      username: 'test_user',
      authenticator: 'EXTERNALBROWSER',
      clientStoreTemporaryCredential: false,
      browserRedirectPort: port,
    });

    if (isRunningNewDriverWithBD('BD#136')) {
      const portInUseOsErrorMessage =
        process.platform === 'win32'
          ? 'Only one usage of each socket address (protocol/network address/port) is normally permitted. (os error 10048)'
          : `Address already in use (os error ${process.platform === 'darwin' ? 48 : 98})`;
      await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject({
        name: 'Error',
        message: `Failed to login: External browser SSO failed: Failed to bind local listener on 127.0.0.1:${port}: ${portInUseOsErrorMessage}`,
      });
    } else {
      await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject({
        name: 'Error',
        message: `Error while creating local server on port ${port}: Error: listen EADDRINUSE: address already in use :::${port}`,
      });
    }
  });
});
