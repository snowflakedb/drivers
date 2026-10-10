import { describe, it, expect } from 'vitest';
import { toCoreConnectionOptions } from '../../src/connection-option-aliases.js';
import { createConnection } from '../../src/index.js';

describe('toCoreConnectionOptions', () => {
  it('maps camelCase driver options onto their sf_core snake_case keys', () => {
    expect(
      toCoreConnectionOptions({
        account: 'sfctest0',
        username: 'alice',
        privateKey: '-----BEGIN PRIVATE KEY-----',
        privateKeyPath: '/tmp/key.p8',
        privateKeyPass: 'secret',
      }),
    ).toEqual({
      account: 'sfctest0',
      user: 'alice',
      private_key: '-----BEGIN PRIVATE KEY-----',
      private_key_file: '/tmp/key.p8',
      private_key_password: 'secret',
    });
  });

  it('should map privateKeyPath onto private_key_file', () => {
    expect(toCoreConnectionOptions({ privateKeyPath: '/key.p8' })).toEqual({
      private_key_file: '/key.p8',
    });
  });

  // TODO(SNOW-3996212): drop these two cases once OAuth and WIF e2e replace them.
  // Kept until then so a map typo on the new aliases fails in unit, not only in Jenkins.
  it('should map the access token used by legacy OAUTH and WORKLOAD_IDENTITY with an OIDC provider', () => {
    expect(
      toCoreConnectionOptions({
        authenticator: 'OAUTH',
        token: 'an.access.token',
      }),
    ).toEqual({
      authenticator: 'OAUTH',
      token: 'an.access.token',
    });
  });

  it('should forward legacy OAUTH without a username', () => {
    expect(
      toCoreConnectionOptions({
        account: 'account',
        authenticator: 'OAUTH',
        token: 'token',
      }),
    ).toEqual({
      account: 'account',
      authenticator: 'OAUTH',
      token: 'token',
    });
  });

  it('should map the workload identity provider', () => {
    expect(
      toCoreConnectionOptions({
        authenticator: 'WORKLOAD_IDENTITY',
        workloadIdentityProvider: 'AWS',
      }),
    ).toEqual({
      authenticator: 'WORKLOAD_IDENTITY',
      workload_identity_provider: 'AWS',
    });
  });

  it('should map certRevocationCheckMode onto crl_check_mode', () => {
    expect(toCoreConnectionOptions({ certRevocationCheckMode: 'ENABLED' })).toEqual({
      crl_check_mode: 'ENABLED',
    });
  });

  it('should map crlAllowCertificatesWithoutCrlURL onto crl_allow_certificates_without_crl_url', () => {
    expect(toCoreConnectionOptions({ crlAllowCertificatesWithoutCrlURL: true })).toEqual({
      crl_allow_certificates_without_crl_url: 'true',
    });
  });

  it('should map crlInMemoryCache onto crl_enable_memory_caching', () => {
    expect(toCoreConnectionOptions({ crlInMemoryCache: false })).toEqual({
      crl_enable_memory_caching: 'false',
    });
  });

  it('should map crlOnDiskCache onto crl_enable_disk_caching', () => {
    expect(toCoreConnectionOptions({ crlOnDiskCache: true })).toEqual({
      crl_enable_disk_caching: 'true',
    });
  });

  it('should convert browserActionTimeout milliseconds to seconds', () => {
    expect(toCoreConnectionOptions({ browserActionTimeout: 120000 })).toEqual({
      browserActionTimeout: '120',
    });
  });

  it.each([
    [0, '0'],
    [-1, '-1'],
    [999, '1'],
  ])(
    'should convert browserActionTimeout %s milliseconds to %s seconds',
    (milliseconds, seconds) => {
      expect(toCoreConnectionOptions({ browserActionTimeout: milliseconds })).toEqual({
        browserActionTimeout: seconds,
      });
    },
  );

  it('should reject a non-numeric browserActionTimeout', () => {
    expect(() =>
      createConnection({
        account: 'test-account',
        username: 'test-user',
        password: 'test-password',
        browserActionTimeout: 'invalid',
      }),
    ).toThrow("Expected type Int for parameter 'browserActionTimeout'");
  });

  it('throws on a key that is not in the alias map', () => {
    expect(() =>
      toCoreConnectionOptions({
        account: 'sfctest0',
        notARealOption: 'x',
      }),
    ).toThrow('Unknown connection option: notARealOption');
  });

  it('returns an empty object for empty input', () => {
    expect(toCoreConnectionOptions({})).toEqual({});
  });
});
