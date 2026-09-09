import { Buffer } from 'node:buffer';
import { describe, expect, it } from 'vitest';
import getTestParameter, { getTestParametersFromSameSource } from '../utils/getTestParameter.js';
import {
  baseConnectionOptions,
  destroyConnectionAsync,
  executeAsync,
  NO_BROWSER_AVAILABLE,
  snowflake,
} from '../utils/index.js';

const OKTA_USER_KEY = 'SNOWFLAKE_TEST_OKTA_USER';
const OKTA_PASSWORD_KEY = 'SNOWFLAKE_TEST_OKTA_PASSWORD';

// The Okta user and its password have to come from one parameters source: a
// user paired with another source's password is a valid-looking request that
// the IdP rejects with HTTP 400.
function requireOktaCredentials(): { user: string; password: string } {
  const credentials = getTestParametersFromSameSource([OKTA_USER_KEY, OKTA_PASSWORD_KEY]);
  if (!credentials) {
    throw new Error(`No parameters source defines both ${OKTA_USER_KEY} and ${OKTA_PASSWORD_KEY}`);
  }
  return {
    user: credentials[OKTA_USER_KEY],
    password: credentials[OKTA_PASSWORD_KEY],
  };
}

async function mintOauthAccessToken(user: string, password: string): Promise<string> {
  const clientId = getTestParameter('SNOWFLAKE_TEST_OKTA_OAUTH_CLIENT_ID', true);
  const clientSecret = getTestParameter('SNOWFLAKE_TEST_OKTA_OAUTH_CLIENT_SECRET', true);
  const role = getTestParameter('SNOWFLAKE_TEST_ROLE', true);
  const response = await fetch(getTestParameter('SNOWFLAKE_TEST_OKTA_OAUTH_TOKEN_URL', true), {
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

describe.skipIf(NO_BROWSER_AVAILABLE)('OAuth authentication', () => {
  it('should authenticate with a pre-acquired access token', async () => {
    // Given Authentication is set to legacy OAUTH and a pre-acquired OAuth access token is supplied via `token=`
    const { user, password } = requireOktaCredentials();
    const token = await mintOauthAccessToken(user, password);
    const connection = snowflake.createConnection({
      ...baseConnectionOptions,
      username: user,
      authenticator: 'OAUTH',
      token,
    });

    try {
      // When Trying to Connect
      await connection.connectAsync();

      // Then Login is successful and a simple query can be executed
      const { rows } = await executeAsync(connection, 'SELECT 1');
      expect(rows).toHaveLength(1);
      expect(Object.values(rows[0])).toEqual([1]);
    } finally {
      await destroyConnectionAsync(connection);
    }
  });
});
