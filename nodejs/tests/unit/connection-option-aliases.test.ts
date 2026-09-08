import { describe, it, expect } from 'vitest';
import { normalizeConnectionOptions } from '../../src/connection-option-aliases.js';

describe('normalizeConnectionOptions', () => {
  it('maps camelCase driver options onto their sf_core snake_case keys', () => {
    expect(
      normalizeConnectionOptions({
        account: 'sfctest0',
        username: 'alice',
        privateKey: '-----BEGIN PRIVATE KEY-----',
        privateKeyPass: 'secret',
      }),
    ).toEqual({
      account: 'sfctest0',
      user: 'alice',
      private_key: '-----BEGIN PRIVATE KEY-----',
      private_key_password: 'secret',
    });
  });

  // TODO(SNOW-3996212): drop these two cases once OAuth and WIF e2e replace them.
  // Kept until then so a map typo on the new aliases fails in unit, not only in Jenkins.
  it('maps the access token used by legacy OAUTH and WORKLOAD_IDENTITY with an OIDC provider', () => {
    expect(
      normalizeConnectionOptions({
        authenticator: 'OAUTH',
        token: 'an.access.token',
      }),
    ).toEqual({
      authenticator: 'OAUTH',
      token: 'an.access.token',
    });
  });

  it('maps the workload identity provider', () => {
    expect(
      normalizeConnectionOptions({
        authenticator: 'WORKLOAD_IDENTITY',
        workloadIdentityProvider: 'AWS',
      }),
    ).toEqual({
      authenticator: 'WORKLOAD_IDENTITY',
      workload_identity_provider: 'AWS',
    });
  });

  it('throws on a key that is not in the alias map', () => {
    expect(() =>
      normalizeConnectionOptions({
        account: 'sfctest0',
        notARealOption: 'x',
      }),
    ).toThrow('Unknown connection option: notARealOption');
  });

  it('returns an empty object for empty input', () => {
    expect(normalizeConnectionOptions({})).toEqual({});
  });
});
