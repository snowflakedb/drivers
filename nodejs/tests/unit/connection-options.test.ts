import { describe, it, expect } from 'vitest';
import { toCoreConnectionOptions } from '../../src/connection-options.js';

describe('toCoreConnectionOptions', () => {
  it('should forward an unknown key so sf_core can warn about it', () => {
    expect(
      toCoreConnectionOptions({
        account: 'sfctest0',
        // @ts-expect-error plain JavaScript callers can pass any key
        notARealOption: 'x',
      }),
    ).toEqual({
      account: 'sfctest0',
      notARealOption: 'x',
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

  it('should drop tokenFilePath when an inline token is set', () => {
    expect(
      toCoreConnectionOptions({
        token: 'inline',
        tokenFilePath: '/tmp/token',
      }),
    ).toEqual({
      token: 'inline',
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
});
