import { describe, expect, it } from 'vitest';
import { normalizeConnectionOptions } from '../../src/normalize-connection-options.js';

describe('normalizeConnectionOptions', () => {
  it('should resolve the user and private_key_file aliases', () => {
    expect(
      normalizeConnectionOptions({
        account: 'testaccount',
        user: 'alice',
        private_key_file: '/key.p8',
      }),
    ).toEqual({ account: 'testaccount', username: 'alice', privateKeyPath: '/key.p8' });
  });

  it('should apply the snake_case-to-camelCase overrides for acronyms', () => {
    expect(
      normalizeConnectionOptions({
        account: 'testaccount',
        client_request_mfa_token: true,
        disable_saml_url_check: true,
        crl_allow_certificates_without_crl_url: true,
      }),
    ).toEqual({
      account: 'testaccount',
      clientRequestMFAToken: true,
      disableSamlURLCheck: true,
      crlAllowCertificatesWithoutCrlURL: true,
    });
  });

  it('should drop undefined values', () => {
    expect(
      normalizeConnectionOptions({
        account: 'testaccount',
        role: undefined,
        warehouse: undefined,
      }),
    ).toEqual({ account: 'testaccount' });
  });

  it('should convert single-word and multi-word snake_case keys', () => {
    expect(
      normalizeConnectionOptions({
        account: 'testaccount',
        warehouse: 'wh',
        database: 'db',
        schema: 'sc',
        role: 'rl',
        private_key_pass: 'snakepass',
        oauth_client_id: 'myid',
        client_session_keep_alive: true,
        workload_identity_provider: 'AWS',
      }),
    ).toEqual({
      account: 'testaccount',
      warehouse: 'wh',
      database: 'db',
      schema: 'sc',
      role: 'rl',
      privateKeyPass: 'snakepass',
      oauthClientId: 'myid',
      clientSessionKeepAlive: true,
      workloadIdentityProvider: 'AWS',
    });
  });

  it('should pass keys that are not lowercase snake_case through unchanged', () => {
    expect(
      normalizeConnectionOptions({
        privateKeyPath: '/camel',
        Private_Key: 'mixed',
        PROXY_HOST: 'upper',
      }),
    ).toEqual({ privateKeyPath: '/camel', Private_Key: 'mixed', PROXY_HOST: 'upper' });
  });

  it('should treat keys named after Object.prototype members as ordinary keys', () => {
    expect(normalizeConnectionOptions({ constructor: 'c', toString: 't' })).toEqual({
      constructor: 'c',
      toString: 't',
    });
  });
});
