import { createRequire } from 'node:module';
import { afterAll, beforeAll, beforeEach, describe, expect, it, onTestFinished } from 'vitest';
import type { Connection, ConnectionOptions } from '../../types/sdk-types.js';
import { acceptUntrustedHttps } from '../utils/fixtures.js';
import getTestParameter from '../utils/getTestParameter.js';
import {
  baseConnectionOptions,
  connectAsyncWithErrorBD,
  IS_RUNNING_FOR_OLD_DRIVER,
  isRunningNewDriverWithBD,
  snowflake,
} from '../utils/index.js';
import { verifySimpleQuery } from '../utils/query.js';
import {
  WiremockServer,
  jsonResponse,
  loginSuccess,
  logoutSuccess,
  type LoggedRequest,
  type StubMapping,
} from '../utils/wiremock/index.js';
import {
  destroyConnectionAfterTest,
  patchOldDriverHttpsProtocol,
  requireOktaCredentials,
} from './utils.js';

describe('native Okta authentication', () => {
  describe('native Okta against wiremock', () => {
    const WIREMOCK_OKTA_URL = 'https://okta.snowflake.com';

    function stubAuthenticatorRequest(tokenUrl: string, ssoUrl: string): StubMapping {
      return {
        request: { method: 'POST', urlPathPattern: '/session/authenticator-request.*' },
        response: jsonResponse(200, { success: true, data: { tokenUrl, ssoUrl } }),
      };
    }

    function createWiremockNativeOktaConnection(
      wiremock: WiremockServer,
      overrides: Partial<ConnectionOptions> = {},
    ): Connection {
      return snowflake.createConnection({
        account: 'testaccount',
        username: 'test_user',
        password: 'test_password',
        authenticator: WIREMOCK_OKTA_URL,
        // Login uses accessUrl from protocol, host, and port. Without protocol the old driver
        // uses https and speaks TLS to the HTTP WireMock port. authenticator-request is redirected
        // separately, because that call ignores host.
        ...(IS_RUNNING_FOR_OLD_DRIVER
          ? {
              // @ts-ignore BD#2 - protocol is not on the old driver ConnectionOptions
              protocol: 'http',
              host: `localhost:${wiremock.port}`,
            }
          : wiremock.connectionOptions),
        ...overrides,
      });
    }

    // The old driver posts authenticator-request to constructHostname(region, account) over the
    // global HTTPS protocol, so it never reaches WireMock. Point both at this server. The
    // returned function puts both CJS singletons back.
    function redirectOldDriverOktaRequestsToWiremock(wiremock: WiremockServer): () => void {
      if (!IS_RUNNING_FOR_OLD_DRIVER) {
        return () => {};
      }
      const restoreHttpsProtocol = patchOldDriverHttpsProtocol('http');
      const restoreConstructHostname = patchOldDriverConstructHostname(
        `localhost:${wiremock.port}`,
      );
      return () => {
        restoreHttpsProtocol();
        restoreConstructHostname();
      };
    }

    // AuthOkta builds the Snowflake host with constructHostname(region, account) and ignores host.
    // The published package does not export a setter, so this replaces the CJS function for the test.
    function patchOldDriverConstructHostname(hostname: string): () => void {
      const util = createRequire(import.meta.url)('snowflake-sdk-old/dist/lib/util.js') as {
        constructHostname: (region?: string, account?: string) => string;
      };
      const oldConstructHostname = util.constructHostname;
      util.constructHostname = () => hostname;
      return () => {
        util.constructHostname = oldConstructHostname;
      };
    }

    describe('IdP URL validation', () => {
      const OTHER_TOKEN_URL = 'https://attacker.snowflake.com/api/v1/authn';

      let wiremock: WiremockServer;

      beforeAll(async () => {
        wiremock = await WiremockServer.spawn();
      });

      beforeEach(async () => {
        await wiremock.reset();
        onTestFinished(redirectOldDriverOktaRequestsToWiremock(wiremock));
      });

      afterAll(async () => {
        await wiremock.destroy();
      });

      it('should reject a token URL on another host', async () => {
        await wiremock.stub(
          stubAuthenticatorRequest(OTHER_TOKEN_URL, `${WIREMOCK_OKTA_URL}/app/sso/saml`),
        );
        const connection = createWiremockNativeOktaConnection(wiremock);

        await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject(
          isRunningNewDriverWithBD('BD#146')
            ? {
                name: 'Error',
                code: undefined,
                message:
                  'Failed to login: Native Okta SSO failed: IdP URL safety validation failed: returned https://attacker.snowflake.com/api/v1/authn does not match configured Okta URL https://okta.snowflake.com/',
              }
            : {
                name: 'Error',
                message:
                  'The prefix of the SSO/token URL and the specified authenticator do not match.',
              },
        );
        expect(connection.isUp()).toBe(false);
      });

      it('should reject a token URL that is not a URL', async () => {
        await wiremock.stub(stubAuthenticatorRequest('abcd', `${WIREMOCK_OKTA_URL}/app/sso/saml`));
        const connection = createWiremockNativeOktaConnection(wiremock);

        await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject(
          isRunningNewDriverWithBD('BD#147')
            ? {
                name: 'Error',
                code: undefined,
                message: 'Failed to login: Native Okta SSO failed: Failed to parse URL (<unknown>)',
              }
            : {
                name: 'Error',
                message: 'Authenticator, SSO, or token URL is invalid.',
              },
        );
        expect(connection.isUp()).toBe(false);
      });
    });

    describe('SAML postback validation', () => {
      const ATTACKER_POSTBACK = 'https://attacker.snowflake.com/fed/login';

      function stubOktaToken(): StubMapping {
        return {
          request: { method: 'POST', urlPathPattern: '/api/v1/authn.*' },
          response: jsonResponse(200, {
            sessionToken: 'okta-session-token',
            relayState: 'relay-state',
          }),
        };
      }

      function stubSamlHtml(html: string): StubMapping {
        return {
          request: { method: 'GET', urlPathPattern: '/app/sso/saml.*' },
          response: { status: 200, headers: { 'Content-Type': 'text/html' }, body: html },
        };
      }

      function createSamlHtml(postback: string, marker: string): string {
        return `<html><body><form method="post" action="${postback}"><input type="hidden" name="SAMLResponse" value="${marker}" /></form></body></html>`;
      }

      function getSamlResponse(request: LoggedRequest): string {
        const parsed = JSON.parse(request.body) as { data: { RAW_SAML_RESPONSE: string } };
        return parsed.data.RAW_SAML_RESPONSE;
      }

      let wiremock: WiremockServer;

      beforeAll(async () => {
        wiremock = await WiremockServer.spawnWithHttps();
      });

      beforeEach(async () => {
        await wiremock.reset();
        onTestFinished(redirectOldDriverOktaRequestsToWiremock(wiremock));
        acceptUntrustedHttps();
      });

      afterAll(async () => {
        await wiremock.destroy();
      });

      function stubOktaFlow(html: string): StubMapping[] {
        const origin = wiremock.httpsUrl;
        return [
          stubAuthenticatorRequest(`${origin}/api/v1/authn`, `${origin}/app/sso/saml`),
          stubOktaToken(),
          stubSamlHtml(html),
        ];
      }

      it('should send the Okta SAML response and AUTHENTICATOR on the login request', async () => {
        const postback = `http://localhost:${wiremock.port}/fed/login`;
        const html = createSamlHtml(postback, 'hop');
        await wiremock.stub([...stubOktaFlow(html), loginSuccess(), logoutSuccess()]);
        const connection = createWiremockNativeOktaConnection(wiremock, {
          authenticator: wiremock.httpsUrl,
        });

        await connection.connectAsync();
        destroyConnectionAfterTest(connection);

        expect(connection.isUp()).toBe(true);
        const [login] = await wiremock.findRequests('/session/v1/login-request.*');
        expect(getSamlResponse(login)).toBe(html);
        const authenticator = (JSON.parse(login.body) as { data: { AUTHENTICATOR?: string } }).data
          .AUTHENTICATOR;
        // BD#153: the old login body omits AUTHENTICATOR for an Okta URL.
        if (isRunningNewDriverWithBD('BD#153')) {
          expect(authenticator).toBe(`${wiremock.httpsUrl}/`);
        } else {
          expect(authenticator).toBeUndefined();
        }
      });

      it('should reject SAML HTML that has no form', async () => {
        await wiremock.stub(stubOktaFlow('<html><body>no form</body></html>'));
        const connection = createWiremockNativeOktaConnection(wiremock, {
          authenticator: wiremock.httpsUrl,
        });

        const expected = `http://localhost:${wiremock.port}:443`;
        await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject(
          isRunningNewDriverWithBD('BD#148')
            ? {
                name: 'Error',
                code: undefined,
                message:
                  'Failed to login: Native Okta SSO failed: Failed to extract SAML postback (form action) from HTML',
              }
            : {
                name: 'Error',
                message: `The specified authenticator and destination URL in the SAML assertion do not match: expected: ${expected} postback: <html><`,
              },
        );
        expect(connection.isUp()).toBe(false);
      });

      it('should reject a SAML postback on another host when the check is on', async () => {
        await wiremock.stub([
          ...stubOktaFlow(createSamlHtml(ATTACKER_POSTBACK, 'attacker')),
          loginSuccess(),
        ]);
        const connection = createWiremockNativeOktaConnection(wiremock, {
          authenticator: wiremock.httpsUrl,
        });

        const expected = `http://localhost:${wiremock.port}`;
        await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject(
          isRunningNewDriverWithBD('BD#149')
            ? {
                name: 'Error',
                code: undefined,
                message: `Failed to login: Native Okta SSO failed: SAML postback destination validation failed: postback ${ATTACKER_POSTBACK} does not match Snowflake server ${expected}`,
              }
            : {
                name: 'Error',
                message: `The specified authenticator and destination URL in the SAML assertion do not match: expected: ${expected}:443 postback: ${ATTACKER_POSTBACK}`,
              },
        );
        expect(await wiremock.findRequests('/session/v1/login-request.*')).toHaveLength(0);
        expect(connection.isUp()).toBe(false);
      });

      it('should connect when disableSamlURLCheck allows another postback host', async () => {
        await wiremock.stub([
          ...stubOktaFlow(createSamlHtml(ATTACKER_POSTBACK, 'attacker')),
          loginSuccess(),
          logoutSuccess(),
        ]);
        const connection = createWiremockNativeOktaConnection(wiremock, {
          authenticator: wiremock.httpsUrl,
          disableSamlURLCheck: true,
        });

        await connection.connectAsync();
        destroyConnectionAfterTest(connection);
        expect(connection.isUp()).toBe(true);
      });

      it('should resend the same SAML body on the new driver and a new body on the old driver', async () => {
        const origin = wiremock.httpsUrl;
        const postback = `http://localhost:${wiremock.port}/fed/login`;
        const first = createSamlHtml(postback, 'first');
        const second = createSamlHtml(postback, 'second');
        const loginFailure: StubMapping = {
          scenarioName: 'native-okta-login',
          requiredScenarioState: 'Started',
          newScenarioState: 'retried',
          request: { method: 'POST', urlPathPattern: '/session/v1/login-request.*' },
          response: { status: 503, body: 'unavailable' },
        };
        const loginRetry: StubMapping = {
          ...loginSuccess(),
          scenarioName: 'native-okta-login',
          requiredScenarioState: 'retried',
        };
        await wiremock.stub([
          stubAuthenticatorRequest(`${origin}/api/v1/authn`, `${origin}/app/sso/saml`),
          stubOktaToken(),
          {
            ...stubSamlHtml(first),
            scenarioName: 'native-okta-saml',
            requiredScenarioState: 'Started',
            newScenarioState: 'second',
          },
          {
            ...stubSamlHtml(second),
            scenarioName: 'native-okta-saml',
            requiredScenarioState: 'second',
          },
          loginFailure,
          loginRetry,
          logoutSuccess(),
        ]);
        const connection = createWiremockNativeOktaConnection(wiremock, { authenticator: origin });

        await connection.connectAsync();
        destroyConnectionAfterTest(connection);

        const samlGets = await wiremock.findRequests('/app/sso/saml.*');
        const loginBodies = (await wiremock.findRequests('/session/v1/login-request.*')).map(
          getSamlResponse,
        );
        if (isRunningNewDriverWithBD('BD#150')) {
          expect(loginBodies).toEqual([first, first]);
          expect(samlGets).toHaveLength(1);
        } else {
          expect(loginBodies).toEqual([first, second]);
          expect(samlGets).toHaveLength(2);
        }
      });
    });
  });

  describe('Live native Okta authentication', { tags: ['requires_auth_test_container'] }, () => {
    const TYPO_OKTA_URL = 'https://testinvalidaccoount.com';
    const INVALID_OKTA_URL = 'https://invalid.okta.com';

    // The old driver posts authenticator-request to constructHostname(region, account) and ignores host.
    function getRegionFromHostForOldOktaRequest(): { region: string } | Record<string, never> {
      if (!IS_RUNNING_FOR_OLD_DRIVER) {
        return {};
      }
      const { account, host } = baseConnectionOptions;
      const prefix = `${account}.`;
      const suffix = '.snowflakecomputing.com';
      if (!account || !host || !host.startsWith(prefix) || !host.endsWith(suffix)) {
        throw new Error(`old driver Okta region cannot be derived from host ${host}`);
      }
      return { region: host.slice(prefix.length, -suffix.length) };
    }

    function createLiveNativeOktaConnection(
      overrides: Partial<ConnectionOptions> = {},
    ): Connection {
      const { user, password } = requireOktaCredentials();
      return snowflake.createConnection({
        ...baseConnectionOptions,
        ...getRegionFromHostForOldOktaRequest(),
        username: user,
        password,
        authenticator: getTestParameter('SNOWFLAKE_TEST_OKTA_URL', true),
        ...overrides,
      });
    }

    it('should authenticate with the Okta URL, user, and password', async () => {
      const connection = createLiveNativeOktaConnection();

      await connection.connectAsync();
      destroyConnectionAfterTest(connection);

      expect(connection.isUp()).toBe(true);
      await verifySimpleQuery(connection);
    });

    it.each([
      ['the password is wrong', { password: 'wrong_password_12345' }],
      [
        'the username and password are both invalid',
        { username: 'itsnotanaccount.com', password: 'fakepassword' },
      ],
    ])('should fail when %s', async (_, overrides) => {
      const connection = createLiveNativeOktaConnection(overrides);

      await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject(
        isRunningNewDriverWithBD('BD#151')
          ? {
              name: 'Error',
              code: undefined,
              message:
                'Failed to login: Native Okta SSO failed: Okta token endpoint rejected credentials (HTTP 401 Unauthorized)',
            }
          : {
              name: 'AxiosError',
              code: 'ERR_BAD_REQUEST',
              message: 'Request failed with status code 401',
            },
      );
      expect(connection.isUp()).toBe(false);
    });

    it('should fail for the old typo Okta URL without reading ssoUrl off null', async () => {
      const connection = createLiveNativeOktaConnection({ authenticator: TYPO_OKTA_URL });
      const connect = connectAsyncWithErrorBD(connection);

      await expect(connect).rejects.toMatchObject(
        isRunningNewDriverWithBD('BD#152')
          ? {
              name: 'Error',
              code: undefined,
              message: 'Failed to login: Native Okta SSO failed: Missing required field: data',
            }
          : {
              name: 'TypeError',
              message: "Cannot read properties of null (reading 'ssoUrl')",
            },
      );
      expect(connection.isUp()).toBe(false);
    });

    it('should fail for https://invalid.okta.com', async () => {
      const connection = createLiveNativeOktaConnection({ authenticator: INVALID_OKTA_URL });
      const connect = connectAsyncWithErrorBD(connection);

      if (isRunningNewDriverWithBD('BD#152')) {
        await expect(connect).rejects.toMatchObject({
          name: 'Error',
          code: undefined,
          message:
            'Failed to login: Native Okta SSO failed: The specified authenticator is not accepted by your Snowflake account configuration.  Please contact your local system administrator to get the correct URL to use.',
        });
      } else {
        // BD#152: the old driver surfaces Snowflake's response, and that sentence is not stable.
        await expect(connect).rejects.toBeInstanceOf(Error);
      }
      expect(connection.isUp()).toBe(false);
    });
  });
});
