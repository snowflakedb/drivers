import { describe, it, expect } from 'vitest';
import { toCoreConnectionOptions } from '../../src/connection-option-aliases.js';
import { createConnection } from '../../src/connection.js';

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

  it('should map jwtTimeout onto jwt_timeout in seconds', () => {
    expect(toCoreConnectionOptions({ jwtTimeout: 600 })).toEqual({
      jwt_timeout: '600',
    });
    expect(toCoreConnectionOptions({ jwtTimeout: 0 })).toEqual({
      jwt_timeout: '0',
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

  it('should stringify a numeric port for sf_core', () => {
    expect(toCoreConnectionOptions({ port: 443, protocol: 'https' })).toEqual({
      port: '443',
      protocol: 'https',
    });
  });

  it('should map certRevocationCheckMode onto crl_check_mode', () => {
    expect(toCoreConnectionOptions({ certRevocationCheckMode: 'ENABLED' })).toEqual({
      crl_check_mode: 'ENABLED',
    });
  });

  it('should map workloadIdentityHost onto wif_host', () => {
    expect(
      toCoreConnectionOptions({
        workloadIdentityHost: 'sts.sc2s.sgov.gov',
      }),
    ).toEqual({
      wif_host: 'sts.sc2s.sgov.gov',
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

  it('should map disableQueryContextCache onto disable_query_context_cache', () => {
    expect(toCoreConnectionOptions({ disableQueryContextCache: true })).toEqual({
      disable_query_context_cache: 'true',
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

  it.each([
    [90000, '90'],
    [999, '1'],
    [0, '0'],
    [-1, '-1'],
  ])('should convert timeout %s milliseconds to %s seconds', (milliseconds, seconds) => {
    expect(toCoreConnectionOptions({ timeout: milliseconds })).toEqual({
      timeout: seconds,
    });
  });

  it.each([
    [10000, '10'],
    [999, '1'],
  ])(
    'should map crlDownloadTimeout %s milliseconds onto crl_http_timeout %s seconds',
    (milliseconds, seconds) => {
      expect(toCoreConnectionOptions({ crlDownloadTimeout: milliseconds })).toEqual({
        crl_http_timeout: seconds,
      });
    },
  );

  it('should map the remaining CRL configure options onto their core parameters', () => {
    expect(
      toCoreConnectionOptions({
        crlDownloadMaxSize: 1048576,
        crlCacheValidityTime: 3600000,
        crlCacheDir: '/tmp/crls',
      }),
    ).toEqual({
      crl_max_download_size: '1048576',
      crl_validity_time: '3600',
      crl_cache_dir: '/tmp/crls',
    });
  });

  it('should reject a non-numeric timeout', () => {
    expect(() =>
      createConnection({
        account: 'test-account',
        username: 'test-user',
        password: 'test-password',
        timeout: 'invalid' as unknown as number,
      }),
    ).toThrow("Expected type Int for parameter 'timeout'");
  });

  it('should reject a non-numeric browserActionTimeout', () => {
    expect(() =>
      createConnection({
        account: 'test-account',
        username: 'test-user',
        password: 'test-password',
        browserActionTimeout: 'invalid' as unknown as number,
      }),
    ).toThrow("Expected type Int for parameter 'browserActionTimeout'");
  });

  it('should reject a non-integer jwtTimeout', () => {
    expect(() =>
      createConnection({
        account: 'test-account',
        username: 'test-user',
        password: 'test-password',
        jwtTimeout: 'invalid' as unknown as number,
      }),
    ).toThrow("Expected type Int for parameter 'jwt_timeout'");
  });

  it('should normalize workload identity option values for sf_core', () => {
    expect(
      toCoreConnectionOptions({
        workloadIdentityImpersonationPath: ['role-a', 'role-b'],
        workloadIdentityAwsUseOutboundToken: true,
      }),
    ).toEqual({
      workload_identity_impersonation_path: 'role-a,role-b',
      workload_identity_aws_use_outbound_token: 'true',
    });
  });

  it('should drop a key that is not in the alias map', () => {
    expect(
      toCoreConnectionOptions({
        account: 'sfctest0',
        // @ts-expect-error plain JavaScript callers can pass any key
        notARealOption: 'x',
      }),
    ).toEqual({
      account: 'sfctest0',
    });
  });

  it('should skip options set to null', () => {
    expect(
      toCoreConnectionOptions({
        account: 'sfctest0',
        password: null as unknown as string,
        timeout: null as unknown as number,
      }),
    ).toEqual({
      account: 'sfctest0',
    });
  });

  it('should copy a PAT password into token when token and tokenFilePath are absent', () => {
    expect(
      toCoreConnectionOptions({
        authenticator: 'PROGRAMMATIC_ACCESS_TOKEN',
        password: 'pat-secret',
      }),
    ).toEqual({
      authenticator: 'PROGRAMMATIC_ACCESS_TOKEN',
      token: 'pat-secret',
    });

    expect(
      toCoreConnectionOptions({
        authenticator: 'programmatic_access_token',
        password: 'pat-secret',
        token: 'inline',
      }),
    ).toEqual({
      authenticator: 'programmatic_access_token',
      password: 'pat-secret',
      token: 'inline',
    });
  });

  it('should map connectionName onto connection_name', () => {
    expect(toCoreConnectionOptions({ connectionName: 'fromFile' })).toEqual({
      connection_name: 'fromFile',
    });
  });

  it('returns an empty object for empty input', () => {
    expect(toCoreConnectionOptions({})).toEqual({});
  });
});
