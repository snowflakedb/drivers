import { Buffer } from 'node:buffer';
import { afterAll, beforeAll, beforeEach, describe, expect, it, onTestFinished } from 'vitest';
import type { Connection, ConnectionOptions } from '../../types/sdk-types.js';
import getTestParameter, { getTestParametersFromSameSource } from '../utils/getTestParameter.js';
import {
  baseConnectionOptions,
  connectAsyncWithErrorBD,
  destroyConnectionAsync,
  IS_RUNNING_FOR_OLD_DRIVER,
  isRunningNewDriverWithBD,
  resetGlobalConfig,
  snowflake,
} from '../utils/index.js';
import { verifySimpleQuery } from '../utils/query.js';
import {
  clientCredentialsTokenStubs,
  jsonResponse,
  loginSuccess,
  logoutSuccess,
  type StubMapping,
  WiremockServer,
} from '../utils/wiremock/index.js';
import {
  cleanBrowserProcesses,
  connectWithBrowserAutomation,
  destroyConnectionAfterTest,
  expectConnectionNotUp,
  provideBrowserCredentials,
  NEVER_ESTABLISHED_QUERY,
  requireOktaCredentials,
  TERMINATED_QUERY,
} from './utils.js';

function formatSingleIssueConfigurationValidationMessage(
  parameter: string,
  detail: string,
): string {
  return `Configuration error: Configuration validation failed (1 issue(s)): ${parameter}: ${detail}`;
}

async function createLiveOauthConnection(
  options: ConnectionOptions,
  shouldCleanupAfterTest = true,
): Promise<Connection> {
  const connection = snowflake.createConnection({ ...options });
  await connection.connectAsync();
  if (shouldCleanupAfterTest) {
    destroyConnectionAfterTest(connection);
  }
  return connection;
}

async function mintOauthAccessToken(user: string, password: string): Promise<string> {
  const clientId = getTestParameter('SNOWFLAKE_TEST_OKTA_OAUTH_CLIENT_ID', true);
  const clientSecret = getTestParameter('SNOWFLAKE_TEST_OKTA_OAUTH_CLIENT_SECRET', true);
  const role = getTestParameter('SNOWFLAKE_TEST_ROLE', true);
  const tokenUrl = getTestParameter('SNOWFLAKE_TEST_OKTA_OAUTH_TOKEN_URL', true);
  const response = await fetch(tokenUrl, {
    method: 'POST',
    headers: {
      Authorization: `Basic ${Buffer.from(`${clientId}:${clientSecret}`).toString('base64')}`,
      'Content-Type': 'application/x-www-form-urlencoded;charset=UTF-8',
    },
    body: new URLSearchParams({
      username: user,
      password,
      grant_type: 'password',
      scope: `session:role:${role.toLowerCase()}`,
    }),
  });

  if (!response.ok) {
    // The IdP reports the reason (invalid_grant, invalid_scope, ...) only in the
    // body; the status alone does not distinguish them.
    throw new Error(
      `OAuth token request failed with HTTP ${response.status}: ${await response.text()}`,
    );
  }

  const payload = (await response.json()) as { access_token?: unknown };
  if (typeof payload.access_token !== 'string' || payload.access_token.length === 0) {
    throw new Error('OAuth token response did not contain an access token');
  }
  return payload.access_token;
}

type AuthorizationCodeCredentials = {
  user: string;
  password: string;
  clientId: string;
  clientSecret: string;
  redirectUri: string;
  totpSeed: string;
};

function requireAuthorizationCodeCredentials(): AuthorizationCodeCredentials {
  const loginKeys = [
    'SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_USER',
    'SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_PASSWORD',
    'SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_MFA_SEED',
  ] as const;
  const loginCredentials = getTestParametersFromSameSource(loginKeys);
  if (!loginCredentials) {
    throw new Error(`No parameters source defines ${loginKeys.join(', ')}`);
  }
  const {
    SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_USER: user,
    SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_PASSWORD: password,
    SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_MFA_SEED: totpSeed,
  } = loginCredentials;
  const clientKeys = [
    'SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_CLIENT_ID',
    'SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_CLIENT_SECRET',
  ] as const;
  const clientCredentials = getTestParametersFromSameSource(clientKeys);
  if (!clientCredentials) {
    throw new Error(`No parameters source defines ${clientKeys.join(', ')}`);
  }
  const {
    SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_CLIENT_ID: clientId,
    SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_CLIENT_SECRET: clientSecret,
  } = clientCredentials;
  return {
    user,
    password,
    totpSeed,
    clientId,
    clientSecret,
    redirectUri: getTestParameter('SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_REDIRECT_URI', true),
  };
}

function createAuthorizationCodeOptions(clientStoreTemporaryCredential: boolean): {
  options: ConnectionOptions;
  credentials: AuthorizationCodeCredentials;
} {
  const credentials = requireAuthorizationCodeCredentials();
  return {
    options: {
      ...baseConnectionOptions,
      username: credentials.user,
      authenticator: 'OAUTH_AUTHORIZATION_CODE',
      oauthClientId: credentials.clientId,
      oauthClientSecret: credentials.clientSecret,
      oauthRedirectUri: credentials.redirectUri,
      clientStoreTemporaryCredential,
      browserActionTimeout: 120000,
    } as ConnectionOptions,
    credentials,
  };
}

function connectWithOauthAuthorizationCode(
  connect: () => Promise<unknown>,
  credentials: Pick<AuthorizationCodeCredentials, 'user' | 'password' | 'totpSeed'>,
): Promise<void> {
  return connectWithBrowserAutomation(connect, () =>
    provideBrowserCredentials(
      'internalOauthSnowflakeSuccess',
      credentials.user,
      credentials.password,
      credentials.totpSeed,
    ),
  );
}

function getClientCredentialsOptions(): ConnectionOptions {
  const credentialKeys = [
    'SNOWFLAKE_TEST_OKTA_OAUTH_EXTERNAL_CLIENT_ID',
    'SNOWFLAKE_TEST_OKTA_OAUTH_EXTERNAL_CLIENT_SECRET',
  ] as const;
  const credentials = getTestParametersFromSameSource(credentialKeys);
  if (!credentials) {
    throw new Error(`No parameters source defines ${credentialKeys.join(', ')}`);
  }
  const {
    SNOWFLAKE_TEST_OKTA_OAUTH_EXTERNAL_CLIENT_ID: oauthClientId,
    SNOWFLAKE_TEST_OKTA_OAUTH_EXTERNAL_CLIENT_SECRET: oauthClientSecret,
  } = credentials;
  const { username, ...withoutUser } = baseConnectionOptions;
  void username;
  return {
    ...withoutUser,
    authenticator: 'OAUTH_CLIENT_CREDENTIALS',
    oauthClientId,
    oauthClientSecret,
    oauthTokenRequestUrl: getTestParameter('SNOWFLAKE_TEST_OKTA_OAUTH_TOKEN_URL', true),
  };
}

describe('tests/definitions/shared/authentication/oauth.feature', () => {
  const FAKE_HOST_CONNECTION_OPTIONS = {
    account: 'test-account',
    host: '127.0.0.1',
    username: 'test-user',
    browserActionTimeout: 2000,
    retryTimeout: 300,
    // TODO(SNOW-3996212): BD#90 — the old driver retries unreachable-host logins
    // (default 7) unless this count is 0. The new driver does not accept the option
    // and should also avoid those retries.
    ...(isRunningNewDriverWithBD('BD#90') ? {} : { sfRetryMaxLoginRetries: 0 }),
  };

  describe('OAuth wrapper validation', () => {
    it('should forward AUTHENTICATOR=OAUTH with TOKEN to core', async () => {
      // Given Authentication is set to legacy OAUTH with a pre-acquired access token
      const options = {
        ...FAKE_HOST_CONNECTION_OPTIONS,
        authenticator: 'OAUTH',
        token: 'fake.jwt.token',
      };

      // When Trying to Connect
      const connection = snowflake.createConnection({ ...options });

      // Then The wrapper forwards the token to sf_core without raising a missing-parameter error for it
      await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject({
        name: 'NetworkError',
        code: 401001,
        message: isRunningNewDriverWithBD('BD#97')
          ? 'Network error. Could not reach Snowflake.'
          : 'connect ECONNREFUSED 127.0.0.1:443',
      });
    });

    it('should fail AUTHENTICATOR=OAUTH when TOKEN is missing', async () => {
      // Given Authentication is set to legacy OAUTH without a TOKEN
      const options = {
        ...FAKE_HOST_CONNECTION_OPTIONS,
        authenticator: 'OAUTH',
      };

      // When Trying to Connect
      const connection = snowflake.createConnection({ ...options });

      // Then Connection fails with a missing-parameter error citing token
      await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject(
        isRunningNewDriverWithBD('BD#100')
          ? {
              name: 'InvalidParameterError',
              code: undefined,
              message: formatSingleIssueConfigurationValidationMessage(
                'token',
                "Missing required parameter 'token' (or 'token_file_path') for OAuth authentication",
              ),
            }
          : {
              name: 'NetworkError',
              code: 401001,
              message: 'connect ECONNREFUSED 127.0.0.1:443',
            },
      );
    });

    it('should accept lowercase oauth authenticator value', async () => {
      // Given Authentication is set to lowercase oauth with a TOKEN
      const options = {
        ...FAKE_HOST_CONNECTION_OPTIONS,
        authenticator: 'oauth',
        token: 'fake.jwt.token',
      };

      // When Trying to Connect
      const connection = snowflake.createConnection({ ...options });

      // Then The wrapper does not reject the AUTHENTICATOR value as unknown
      await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject({
        name: 'NetworkError',
        code: 401001,
        message: isRunningNewDriverWithBD('BD#97')
          ? 'Network error. Could not reach Snowflake.'
          : 'connect ECONNREFUSED 127.0.0.1:443',
      });
    });

    it('should fail when AUTHENTICATOR is an unknown OAuth-like value', async () => {
      // Given Authentication is set to a typo of an OAuth flow name
      const options = {
        ...FAKE_HOST_CONNECTION_OPTIONS,
        authenticator: 'OAUTH_AUTHORIZATION_TYPO',
      };

      // When Trying to Connect
      const connection = snowflake.createConnection({ ...options });

      // Then Connection fails with an authenticator-related error
      await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject(
        isRunningNewDriverWithBD('BD#99')
          ? {
              name: 'InvalidParameterError',
              code: undefined,
              message: formatSingleIssueConfigurationValidationMessage(
                'authenticator',
                "Invalid authenticator 'OAUTH_AUTHORIZATION_TYPO'. Allowed values are snowflake, snowflake_jwt, snowflake_password, programmatic_access_token, pat_with_external_session, username_password_mfa, externalbrowser, oauth, oauth_client_credentials, oauth_authorization_code, workload_identity or an https:// URL for native Okta SSO (case-insensitive)",
              ),
            }
          : {
              name: 'NetworkError',
              code: 401001,
              message: 'connect ECONNREFUSED 127.0.0.1:443',
            },
      );
    });

    it('should fail OAUTH_CLIENT_CREDENTIALS when client_id is missing', async () => {
      // Given Authentication is set to OAUTH_CLIENT_CREDENTIALS without oauth_client_id
      const options = {
        ...FAKE_HOST_CONNECTION_OPTIONS,
        authenticator: 'OAUTH_CLIENT_CREDENTIALS',
        oauthClientSecret: 'test-client-secret',
        oauthTokenRequestUrl: 'https://127.0.0.1/oauth/token',
      } as ConnectionOptions;

      // When Trying to Connect
      if (isRunningNewDriverWithBD('BD#101')) {
        const connection = snowflake.createConnection({ ...options });

        // Then Connection fails with a missing-parameter error citing oauth_client_id
        await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject({
          name: 'InvalidParameterError',
          code: undefined,
          message: formatSingleIssueConfigurationValidationMessage(
            'oauth_client_id',
            "Missing required parameter 'oauth_client_id' for OAuth client credentials authentication",
          ),
        });
      } else {
        // Then Connection fails with a missing-parameter error citing oauth_client_id
        expect(() => snowflake.createConnection({ ...options })).toThrow(
          expect.objectContaining({
            name: 'InvalidParameterError',
            code: 404059,
            message: 'Invalid oauth client id. The specified value must not be an empty string',
          }),
        );
      }
    });

    it('should fail OAUTH_CLIENT_CREDENTIALS when client_secret is missing', async () => {
      // Given Authentication is set to OAUTH_CLIENT_CREDENTIALS without oauth_client_secret
      const options = {
        ...FAKE_HOST_CONNECTION_OPTIONS,
        authenticator: 'OAUTH_CLIENT_CREDENTIALS',
        oauthClientId: 'test-client-id',
        oauthTokenRequestUrl: 'https://127.0.0.1/oauth/token',
      } as ConnectionOptions;

      // When Trying to Connect
      if (isRunningNewDriverWithBD('BD#101')) {
        const connection = snowflake.createConnection({ ...options });

        // Then Connection fails with a missing-parameter error citing oauth_client_secret
        await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject({
          name: 'InvalidParameterError',
          code: undefined,
          message: formatSingleIssueConfigurationValidationMessage(
            'oauth_client_secret',
            "Missing required parameter 'oauth_client_secret' for OAuth client credentials authentication",
          ),
        });
      } else {
        // Then Connection fails with a missing-parameter error citing oauth_client_secret
        expect(() => snowflake.createConnection({ ...options })).toThrow(
          expect.objectContaining({
            name: 'InvalidParameterError',
            code: 404060,
            message: 'Invalid oauth client secret. The specified value must not be an empty string',
          }),
        );
      }
    });

    it('should fail OAUTH_CLIENT_CREDENTIALS when token_request_url is missing', async () => {
      // Given Authentication is set to OAUTH_CLIENT_CREDENTIALS without oauth_token_request_url
      const options = {
        ...FAKE_HOST_CONNECTION_OPTIONS,
        authenticator: 'OAUTH_CLIENT_CREDENTIALS',
        oauthClientId: 'test-client-id',
        oauthClientSecret: 'test-client-secret',
      } as ConnectionOptions;

      // When Trying to Connect
      const connection = snowflake.createConnection({ ...options });

      // Then Connection fails with a missing-parameter error citing oauth_token_request_url
      await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject(
        isRunningNewDriverWithBD('BD#101')
          ? {
              name: 'InvalidParameterError',
              code: undefined,
              message: formatSingleIssueConfigurationValidationMessage(
                'oauth_token_request_url',
                "Missing required parameter 'oauth_token_request_url' for OAuth client credentials authentication",
              ),
            }
          : {
              name: 'Error',
              code: 'ECONNREFUSED',
              message: 'connect ECONNREFUSED 127.0.0.1:443',
            },
      );
    });

    it('should fail OAUTH_CLIENT_CREDENTIALS when token_request_url uses a file scheme', async () => {
      // The old driver rejects this at createConnection with code 404061.
      const options = {
        ...FAKE_HOST_CONNECTION_OPTIONS,
        authenticator: 'OAUTH_CLIENT_CREDENTIALS',
        oauthClientId: 'test',
        oauthClientSecret: 'secretValue',
        oauthTokenRequestUrl: 'file://test.app',
      } as ConnectionOptions;

      if (isRunningNewDriverWithBD('BD#101')) {
        const connection = snowflake.createConnection({ ...options });
        await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject({
          name: 'InvalidParameterError',
          code: undefined,
          message:
            "Configuration error: Invalid value 'file://test.app/' for parameter 'oauth_token_request_url' - OAuth endpoint must use https (http is permitted only for loopback hosts)",
        });
      } else {
        expect(() => snowflake.createConnection({ ...options })).toThrow(
          expect.objectContaining({
            name: 'InvalidParameterError',
            code: 404061,
            message:
              'Invalid oauth token request URL. The specified value must be a valid URL starting with the https or http protocol.',
          }),
        );
      }
    });

    it('should not echo OAUTH_CLIENT_SECRET in diagnostics', async () => {
      // Given Authentication is set to OAUTH_CLIENT_CREDENTIALS with a distinctive client secret literal
      const secret = 'ZZ_NODE_SECRET_NEEDLE_OAUTH_CC_ZZ';
      const options = {
        ...FAKE_HOST_CONNECTION_OPTIONS,
        authenticator: 'OAUTH_CLIENT_CREDENTIALS',
        oauthClientId: 'test-client-id',
        oauthClientSecret: secret,
        oauthTokenRequestUrl: 'https://127.0.0.1/oauth/token',
      } as ConnectionOptions;

      // When Trying to Connect
      const connection = snowflake.createConnection({ ...options });
      const error = await connectAsyncWithErrorBD(connection).catch((caught: unknown) => caught);
      if (!(error instanceof Error)) {
        throw new Error('Expected connection attempt to fail');
      }

      // Then No diagnostic record contains the literal client secret
      let diagnostic = '';
      for (let current: unknown = error; current instanceof Error; current = current.cause) {
        diagnostic += `${current.name}: ${current.message}\n`;
      }
      expect(diagnostic).not.toContain(secret);
    });
  });

  function createWiremockClientCredentialsConnection(wiremock: WiremockServer): Connection {
    return snowflake.createConnection({
      ...FAKE_HOST_CONNECTION_OPTIONS,
      authenticator: 'OAUTH_CLIENT_CREDENTIALS',
      role: 'ANALYST',
      oauthClientId: '123',
      oauthClientSecret: 'clientSecret',
      oauthTokenRequestUrl: `http://localhost:${wiremock.port}/oauth/token-request`,
      ...(IS_RUNNING_FOR_OLD_DRIVER ? { oauthHttpAllowed: true } : {}),
    });
  }

  describe('client credentials token endpoint', () => {
    let wiremock: WiremockServer;

    beforeAll(async () => {
      wiremock = await WiremockServer.spawn();
    });

    beforeEach(async () => {
      await wiremock.reset();
    });

    afterAll(async () => {
      await wiremock.destroy();
    });

    it('should omit scope from the client credentials token request when role and scope are unset', async () => {
      await wiremock.stub([
        ...clientCredentialsTokenStubs(200, {
          access_token: 'cc-access-token',
          token_type: 'Bearer',
          expires_in: 600,
        }),
        loginSuccess(),
        logoutSuccess(),
      ]);
      const connection = snowflake.createConnection({
        ...FAKE_HOST_CONNECTION_OPTIONS,
        ...wiremock.connectionOptions,
        authenticator: 'OAUTH_CLIENT_CREDENTIALS',
        oauthClientId: '123',
        oauthClientSecret: 'clientSecret',
        oauthTokenRequestUrl: `http://localhost:${wiremock.port}/oauth/token-request`,
        ...(IS_RUNNING_FOR_OLD_DRIVER ? { oauthHttpAllowed: true } : {}),
      });
      await connection.connectAsync();
      destroyConnectionAfterTest(connection);

      const requests = await wiremock.findRequests('/oauth/token-request.*');
      expect(requests.length).toBeGreaterThan(0);
      for (const request of requests) {
        expect(request.body).not.toContain('scope=');
      }
    });

    it('should derive the client credentials token scope from role', async () => {
      await wiremock.stub([
        ...clientCredentialsTokenStubs(
          200,
          {
            access_token: 'cc-access-token',
            token_type: 'Bearer',
            expires_in: 600,
          },
          'session:role:ANALYST',
        ),
        loginSuccess(),
        logoutSuccess(),
      ]);
      const connection = snowflake.createConnection({
        ...FAKE_HOST_CONNECTION_OPTIONS,
        ...wiremock.connectionOptions,
        authenticator: 'OAUTH_CLIENT_CREDENTIALS',
        role: 'ANALYST',
        oauthClientId: '123',
        oauthClientSecret: 'clientSecret',
        oauthTokenRequestUrl: `http://localhost:${wiremock.port}/oauth/token-request`,
        ...(IS_RUNNING_FOR_OLD_DRIVER ? { oauthHttpAllowed: true } : {}),
      });
      await connection.connectAsync();
      destroyConnectionAfterTest(connection);

      const requests = await wiremock.findRequests('/oauth/token-request.*');
      expect(requests.length).toBeGreaterThan(0);
      for (const request of requests) {
        const params = new URLSearchParams(request.body);
        expect(params.get('scope')).toBe('session:role:ANALYST');
      }
    });

    it('should fail client credentials when the token response omits access_token', async () => {
      await wiremock.stub(
        clientCredentialsTokenStubs(
          200,
          {
            token_type: 'Bearer',
            username: 'user',
            scope: 'refresh_token session:role:ANALYST',
            expires_in: 600,
            refresh_token_expires_in: 86399,
            idpInitiated: false,
          },
          'session:role:ANALYST',
        ),
      );
      await expect(
        connectAsyncWithErrorBD(createWiremockClientCredentialsConnection(wiremock)),
      ).rejects.toMatchObject(
        isRunningNewDriverWithBD('BD#138')
          ? {
              name: 'Error',
              code: undefined,
              message: 'Failed to login: OAuth flow failed: Failed to decode OAuth token response',
            }
          : {
              name: 'OperationProcessingError',
              code: 'OAUTH_INVALID_RESPONSE',
              message: '"response" body "access_token" property must be a string',
            },
      );
    });

    it('should fail client credentials when the token endpoint returns invalid_client', async () => {
      await wiremock.stub(
        clientCredentialsTokenStubs(
          400,
          {
            error: 'invalid_client',
            error_description: 'The client secret supplied for a confidential client is invalid.',
          },
          'session:role:ANALYST',
        ),
      );
      await expect(
        connectAsyncWithErrorBD(createWiremockClientCredentialsConnection(wiremock)),
      ).rejects.toMatchObject(
        isRunningNewDriverWithBD('BD#138')
          ? {
              name: 'Error',
              code: undefined,
              message:
                'Failed to login: OAuth flow failed: Identity Provider responded with error: invalid_client: The client secret supplied for a confidential client is invalid.',
            }
          : {
              name: 'AxiosError',
              code: 'ERR_BAD_REQUEST',
              message: 'Request failed with status code 400',
            },
      );
    });
  });

  describe('Live OAuth authentication', { tags: ['requires_auth_test_container'] }, () => {
    it('oauth should authenticate with pre acquired access token', async () => {
      // Given Authentication is set to legacy OAUTH and a pre-acquired OAuth access token is supplied via `token=`
      const { user, password } = requireOktaCredentials();
      const token = await mintOauthAccessToken(user, password);

      // When Trying to Connect
      const connection = await createLiveOauthConnection({
        ...baseConnectionOptions,
        username: user,
        authenticator: 'OAUTH',
        token,
      });

      // Then Login is successful and a simple query can be executed
      await verifySimpleQuery(connection);
    });

    // Python, JDBC, and ODBC skip this scenario so a junk TOKEN= login does not lock the
    // shared account. Core ignores every OAuth e2e test behind auth_oauth_e2e.
    it.skip('oauth should fail legacy authentication with invalid token', async () => {
      // Given Authentication is set to legacy OAUTH and an invalid OAuth access token is supplied
      const { user } = requireOktaCredentials();
      const connection = snowflake.createConnection({
        ...baseConnectionOptions,
        username: user,
        authenticator: 'OAUTH',
        token: 'invalid_oauth_token_12345',
      });

      // When Trying to Connect
      // Then Connection fails with an authentication / login error
      await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject({
        message: expect.stringMatching(/invalid oauth access token/i),
      });

      // And A subsequent query fails because the connection is not usable
      await expectConnectionNotUp(connection);
    });

    it('oauth should authenticate using authorization code flow', async () => {
      // Given Authentication is set to OAUTH_AUTHORIZATION_CODE with a valid client id / secret
      const { options, credentials } = createAuthorizationCodeOptions(false);
      const connection = snowflake.createConnection({ ...options });
      await cleanBrowserProcesses();

      // When Trying to Connect (this will spawn the local-loopback HTTP listener and `xdg-open`/`open`/`ShellExecute` the IdP login URL unless a previously cached access token short-circuits the leg)
      await connectWithOauthAuthorizationCode(() => connection.connectAsync(), credentials);
      destroyConnectionAfterTest(connection);

      // Then Login is successful and a simple query can be executed
      await verifySimpleQuery(connection);
    });

    it('oauth should reuse cached access token without browser interaction', async () => {
      const { options, credentials } = createAuthorizationCodeOptions(true);
      const browserLoginConnection = snowflake.createConnection({ ...options });
      await cleanBrowserProcesses();
      await connectWithOauthAuthorizationCode(
        () => browserLoginConnection.connectAsync(),
        credentials,
      );
      onTestFinished(async () => {
        await destroyConnectionAsync(browserLoginConnection);
      });
      await verifySimpleQuery(browserLoginConnection);

      // Given Authentication is set to OAUTH_AUTHORIZATION_CODE with client_store_temporary_credential=true and a token has been cached from a previous browser authentication

      const cachedTokenConnection = snowflake.createConnection({
        ...({
          ...options,
          openExternalBrowserCallback: () => {
            throw new Error('Browser should not be launched');
          },
        } as ConnectionOptions),
      });
      // When Trying to Connect without browser interaction
      await cachedTokenConnection.connectAsync();
      destroyConnectionAfterTest(cachedTokenConnection);

      // Then Login is successful and a simple query can be executed
      await verifySimpleQuery(cachedTokenConnection);
    });

    describe.skip('should avoid bad-secret account lockouts', () => {
      it('oauth should fail authorization code flow with bad client secret', async () => {
        // Given Authentication is set to OAUTH_AUTHORIZATION_CODE with a valid client id but a deliberately invalid client secret
        const { options, credentials } = createAuthorizationCodeOptions(false);
        const connection = snowflake.createConnection({
          ...({
            ...options,
            oauthClientSecret: 'invalid_client_secret_12345',
          } as ConnectionOptions),
        });
        await cleanBrowserProcesses();

        // When Trying to Connect
        const connecting = connectWithOauthAuthorizationCode(
          () => connection.connectAsync(),
          credentials,
        );

        // Then Connection fails with an authentication / login error
        await expect(connecting).rejects.toMatchObject({
          message: expect.stringContaining('invalid_client'),
        });
      });
    });

    // Skipped: bad-secret tests cause pipeline flakiness by blocking the test account.
    it.skip('oauth should fail client credentials flow with bad client secret', async () => {
      // Given Authentication is set to OAUTH_CLIENT_CREDENTIALS with a valid client id, an invalid client secret and a valid token_request_url
      const options = getClientCredentialsOptions();
      const connection = snowflake.createConnection({
        ...({
          ...options,
          username: options.oauthClientId,
          oauthClientSecret: 'invalid_client_secret_12345',
        } as ConnectionOptions),
      });

      // When Trying to Connect
      const connecting = connectAsyncWithErrorBD(connection);

      // Then Connection fails with an authentication / login error
      await expect(connecting).rejects.toMatchObject(
        isRunningNewDriverWithBD('BD#138')
          ? {
              name: 'Error',
              code: undefined,
              message: expect.stringContaining('invalid_client'),
            }
          : {
              name: 'AxiosError',
              code: 'ERR_BAD_REQUEST',
              message: 'Request failed with status code 400',
            },
      );
      await expectConnectionNotUp(
        connection,
        isRunningNewDriverWithBD('BD#61') ? TERMINATED_QUERY : NEVER_ESTABLISHED_QUERY,
      );
    });

    it('oauth should authenticate using lowercase oauth authenticator', async () => {
      // Given Authentication is set to lowercase oauth and a valid pre-acquired OAuth access token is supplied via TOKEN
      const { user, password } = requireOktaCredentials();

      // When Trying to Connect
      const connection = await createLiveOauthConnection({
        ...baseConnectionOptions,
        username: user,
        authenticator: 'oauth',
        token: await mintOauthAccessToken(user, password),
      });

      // Then Login is successful and a simple query can be executed
      await verifySimpleQuery(connection);
    });

    it('should authenticate with pre acquired access token without user', async () => {
      // Given Authentication is set to legacy OAUTH and a pre-acquired OAuth access token is supplied via `token=` and user is omitted
      const { user, password } = requireOktaCredentials();
      // SNOWFLAKE_TEST_USER must not ride along: the token identifies the user.
      const { username, ...connectionOptionsWithoutUser } = baseConnectionOptions;
      void username;

      // When Trying to Connect without user
      const connection = await createLiveOauthConnection({
        ...connectionOptionsWithoutUser,
        authenticator: 'OAUTH',
        token: await mintOauthAccessToken(user, password),
      });

      // Then Login is successful and a simple query can be executed
      await verifySimpleQuery(connection);
    });

    it('oauth should authenticate using client credentials flow', async () => {
      // Given Authentication is set to OAUTH_CLIENT_CREDENTIALS with a valid client id / secret and an external IdP token URL. Snowflake's GS does not mint CC tokens, so `oauth_token_request_url` is required up-front.
      const options = getClientCredentialsOptions();

      // When Trying to Connect
      const connection = await createLiveOauthConnection({
        ...options,
        username: options.oauthClientId,
      });

      // Then Login is successful and a simple query can be executed
      await verifySimpleQuery(connection);
    });

    it('should authenticate using client credentials flow without user', async () => {
      // Given Authentication is set to OAUTH_CLIENT_CREDENTIALS with a valid client id / secret and an external IdP token URL and user is omitted
      const options = getClientCredentialsOptions();

      // When Trying to Connect without user
      const connection = await createLiveOauthConnection(options);

      // Then Login is successful and a simple query can be executed
      await verifySimpleQuery(connection);
    });

    it('should reject an invalid client ID in Client Credentials flow', async () => {
      const options = getClientCredentialsOptions();
      const connection = snowflake.createConnection({
        ...({
          ...options,
          username: options.oauthClientId,
          oauthClientId: 'invalidClientId',
        } as ConnectionOptions),
      });
      // TODO(SNOW-4350899): add better errors for invalid client_id errors.
      // Follow-up: https://github.com/snowflakedb/drivers/pull/2897
      await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject(
        isRunningNewDriverWithBD('BD#138')
          ? {
              name: 'Error',
              code: undefined,
              message: 'Failed to login: OAuth flow failed: Failed to decode OAuth token response',
            }
          : {
              name: 'AxiosError',
              code: 'ERR_BAD_REQUEST',
              message: 'Request failed with status code 400',
            },
      );
      await expectConnectionNotUp(
        connection,
        isRunningNewDriverWithBD('BD#61') ? TERMINATED_QUERY : NEVER_ESTABLISHED_QUERY,
      );
    });

    it('should reject a mismatched username for a pre-acquired token', async () => {
      // Given Authentication is set to a pre-acquired token with a username that is not the user tied to the access token
      const { user, password } = requireOktaCredentials();
      const connection = snowflake.createConnection({
        ...baseConnectionOptions,
        username: 'itsnotanaccount.com',
        authenticator: 'OAUTH',
        token: await mintOauthAccessToken(user, password),
      });

      // When Trying to Connect
      const connecting = connectAsyncWithErrorBD(connection);

      // Then Connection fails with "The user you were trying to authenticate as differs from the user tied to the access token."
      await expect(connecting).rejects.toMatchObject(
        isRunningNewDriverWithBD('BD#98')
          ? {
              name: 'OperationFailedError',
              code: '390309',
              message: expect.stringMatching(
                /^Failed to login: Login error: The user you were trying to authenticate as differs from the user tied to the access token\. .+, code: 390309$/,
              ),
            }
          : {
              name: 'OperationFailedError',
              code: '390309',
              message: expect.stringMatching(
                /^The user you were trying to authenticate as differs from the user tied to the access token\. .+$/,
              ),
            },
      );
      await expectConnectionNotUp(connection);
    });
  });
});

it(
  'should time out authorization code when browser credentials are not provided',
  { tags: ['requires_auth_test_container'] },
  async () => {
    const { options } = createAuthorizationCodeOptions(false);
    const connection = snowflake.createConnection({
      ...({
        ...options,
        browserActionTimeout: 2000,
      } as ConnectionOptions),
    });
    onTestFinished(cleanBrowserProcesses);
    await cleanBrowserProcesses();
    await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject(
      isRunningNewDriverWithBD('BD#95')
        ? { message: expect.stringContaining('OAuth browser authorization timed out') }
        : { message: expect.stringContaining('Browser action timed out after 2000 ms.') },
    );
    await expectConnectionNotUp(
      connection,
      isRunningNewDriverWithBD('BD#61') ? TERMINATED_QUERY : NEVER_ESTABLISHED_QUERY,
    );
  },
);

describe('authorization code against wiremock', () => {
  const SINGLE_USE_REFRESH_STUBS: StubMapping[] = [
    {
      scenarioName: 'single-use refresh',
      requiredScenarioState: 'Started',
      newScenarioState: 'refresh token stored',
      request: {
        method: 'POST',
        urlPathPattern: '/oauth/token-request.*',
        bodyPatterns: [{ contains: 'grant_type=authorization_code' }],
      },
      response: jsonResponse(200, {
        access_token: 'AT-FIRST',
        refresh_token: 'RT-FIRST',
        token_type: 'Bearer',
        expires_in: 600,
      }),
    },
    {
      scenarioName: 'single-use refresh',
      requiredScenarioState: 'refresh token stored',
      request: {
        method: 'POST',
        urlPathPattern: '/oauth/token-request.*',
        bodyPatterns: [{ contains: 'grant_type=refresh_token' }],
      },
      response: jsonResponse(200, {
        access_token: 'AT-NEW',
        token_type: 'Bearer',
        expires_in: 600,
      }),
    },
    {
      scenarioName: 'single-use login',
      requiredScenarioState: 'Started',
      newScenarioState: 'access token expired',
      request: { method: 'POST', urlPathPattern: '/session/v1/login-request.*' },
      response: jsonResponse(200, {
        success: false,
        code: '390318',
        message: 'OAuth access token has expired.',
      }),
    },
    {
      ...loginSuccess(),
      scenarioName: 'single-use login',
      requiredScenarioState: 'access token expired',
      newScenarioState: 'logged in, refresh token dropped',
    },
    logoutSuccess(),
  ];

  const EMPTY_TOKEN_RESPONSE_STUB: StubMapping = {
    request: {
      method: 'POST',
      urlPathPattern: '/oauth/token-request.*',
      bodyPatterns: [{ contains: 'code=invalidCode' }],
    },
    response: {
      status: 400,
      headers: { 'Content-Type': 'application/json' },
      body: '',
    },
  };

  function createLoopbackRedirect(options: {
    code: string;
    state?: string;
  }): (authorizeUrl: string) => Promise<void> {
    return async (authorizeUrl) => {
      const authorize = new URL(authorizeUrl);
      const redirectUri = authorize.searchParams.get('redirect_uri');
      const state = options.state ?? authorize.searchParams.get('state');
      if (!redirectUri || !state) {
        throw new Error('authorize URL missing redirect_uri or state');
      }
      const redirect = new URL(redirectUri);
      redirect.searchParams.set('code', options.code);
      redirect.searchParams.set('state', state);
      const response = await fetch(redirect);
      if (!response.ok) {
        throw new Error(`loopback redirect failed: ${response.status}`);
      }
    };
  }

  const AUTHORIZATION_CODE_WIREMOCK_BASE = {
    account: 'testaccount',
    username: 'alice',
    authenticator: 'OAUTH_AUTHORIZATION_CODE' as const,
    oauthClientId: 'test-oauth-client-id',
    oauthClientSecret: 'test-oauth-client-secret',
    oauthAuthorizationUrl: 'https://account.snowflakecomputing.com/oauth/authorize',
  };

  let wiremock: WiremockServer;

  beforeEach(async () => {
    wiremock = await WiremockServer.spawn();
    onTestFinished(() => wiremock.destroy());
  });

  it('should drop the cached refresh token when a refresh response omits one', async () => {
    onTestFinished(() => resetGlobalConfig());
    const tokens = new Map<string, string>();
    const removedRefreshTokens: string[] = [];
    snowflake.configure({
      customCredentialManager: {
        read(key) {
          return tokens.get(key) ?? null;
        },
        write(key, token) {
          tokens.set(key, token);
        },
        remove(key) {
          if (key.includes('OauthRefreshToken')) {
            removedRefreshTokens.push(tokens.get(key) ?? '');
          }
          tokens.delete(key);
        },
      },
    });
    await wiremock.stub(SINGLE_USE_REFRESH_STUBS);

    const connection = snowflake.createConnection({
      ...AUTHORIZATION_CODE_WIREMOCK_BASE,
      oauthTokenRequestUrl: `${wiremock.url}/oauth/token-request`,
      // BD#141: the old driver rejects an http token URL unless this testing option is set.
      ...(isRunningNewDriverWithBD('BD#141') ? {} : { oauthHttpAllowed: true }),
      oauthEnableSingleUseRefreshTokens: true,
      clientStoreTemporaryCredential: true,
      openExternalBrowserCallback: createLoopbackRedirect({ code: 'THE-CODE' }),
      ...wiremock.connectionOptions,
    });
    onTestFinished(() => destroyConnectionAsync(connection));

    await connection.connectAsync();

    const loginRequests = await wiremock.findRequests('/session/v1/login-request.*');
    expect(JSON.parse(loginRequests[0].body).data.TOKEN).toBe('AT-FIRST');
    expect(JSON.parse(loginRequests[1].body).data.TOKEN).toBe('AT-NEW');
    const refreshTokenStillCached = [...tokens.entries()].some(
      ([key, value]) => key.includes('OauthRefreshToken') && value === 'RT-FIRST',
    );
    // BD#144: a successful refresh that omits the refresh token deletes it only on the new driver.
    if (isRunningNewDriverWithBD('BD#144')) {
      expect(removedRefreshTokens).toEqual(['RT-FIRST']);
      expect(refreshTokenStillCached).toBe(false);
    } else {
      expect(removedRefreshTokens).toEqual([]);
      expect(refreshTokenStillCached).toBe(true);
    }
  });

  it('should reject a redirect whose state does not match', async () => {
    const connection = snowflake.createConnection({
      ...AUTHORIZATION_CODE_WIREMOCK_BASE,
      oauthTokenRequestUrl: `${wiremock.url}/oauth/token-request`,
      openExternalBrowserCallback: createLoopbackRedirect({
        code: 'THE-CODE',
        state: 'invalidState',
      }),
      ...wiremock.connectionOptions,
    });

    await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject(
      isRunningNewDriverWithBD('BD#143')
        ? {
            name: 'Error',
            code: undefined,
            message:
              'Failed to login: OAuth flow failed: Identity Provider did not provide expected state parameter! It might indicate an XSS attack.',
          }
        : {
            name: 'OperationProcessingError',
            code: 'OAUTH_INVALID_RESPONSE',
            message: 'unexpected "state" response parameter value',
          },
    );
    expect(await wiremock.findRequests('/oauth/token-request.*')).toEqual([]);
    expect(await wiremock.findRequests('/session/v1/login-request.*')).toEqual([]);
  });

  it('should reject an empty 400 from the token endpoint', async () => {
    await wiremock.stub(EMPTY_TOKEN_RESPONSE_STUB);
    const connection = snowflake.createConnection({
      ...AUTHORIZATION_CODE_WIREMOCK_BASE,
      oauthTokenRequestUrl: `${wiremock.url}/oauth/token-request`,
      // BD#141: the old driver rejects an http token URL unless this testing option is set.
      ...(isRunningNewDriverWithBD('BD#141') ? {} : { oauthHttpAllowed: true }),
      openExternalBrowserCallback: createLoopbackRedirect({ code: 'invalidCode' }),
      ...wiremock.connectionOptions,
    });

    await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject(
      isRunningNewDriverWithBD('BD#142')
        ? {
            name: 'Error',
            code: undefined,
            message:
              'Failed to login: OAuth flow failed: Identity Provider responded with error: unknown: server returned empty error response',
          }
        : {
            name: 'AxiosError',
            code: 'ERR_BAD_REQUEST',
            message: 'Request failed with status code 400',
          },
    );
    expect(await wiremock.findRequests('/oauth/token-request.*')).toHaveLength(1);
    expect(await wiremock.findRequests('/session/v1/login-request.*')).toEqual([]);
  });
});

describe('sfRetryMaxLoginRetries', () => {
  it('should reject sfRetryMaxLoginRetries as an unknown option', () => {
    const options = {
      account: 'test-account',
      username: 'test-user',
      authenticator: 'OAUTH',
      token: 'fake.jwt.token',
      sfRetryMaxLoginRetries: 0,
    };
    if (isRunningNewDriverWithBD('BD#90')) {
      expect(() => snowflake.createConnection({ ...options })).toThrow(
        'Unknown connection option: sfRetryMaxLoginRetries',
      );
    } else {
      snowflake.createConnection({ ...options });
    }
  });
});
