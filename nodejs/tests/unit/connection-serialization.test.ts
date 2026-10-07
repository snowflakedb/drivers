import { describe, expect, it } from 'vitest';
import { deserializeTokenInfo, serializeTokenInfo } from '../../src/connection-serialization.js';
import ErrorMessage from '../../src/constants/ErrorMessage.js';

describe('connection serialization', () => {
  it('should map core expiry fields onto the legacy payload names', () => {
    expect(
      JSON.parse(
        serializeTokenInfo({
          sessionToken: 'session',
          masterToken: 'master',
          sessionTokenExpiresAtMs: 2_000,
          masterTokenExpiresAtMs: 3_000,
        }),
      ),
    ).toEqual({
      services: {
        sf: {
          tokenInfo: {
            sessionToken: 'session',
            masterToken: 'master',
            sessionTokenExpirationTime: 2_000,
            masterTokenExpirationTime: 3_000,
          },
        },
      },
    });
  });

  it('should serialize missing token info as an empty object', () => {
    expect(serializeTokenInfo(null)).toBe(JSON.stringify({ services: { sf: { tokenInfo: {} } } }));
  });

  it('should restore both tokens and their expiration times from a serialize payload', () => {
    expect(
      deserializeTokenInfo(
        serializeTokenInfo({
          sessionToken: 'session',
          masterToken: 'master',
          sessionTokenExpiresAtMs: 2_000,
          masterTokenExpiresAtMs: 3_000,
        }),
      ),
    ).toEqual({
      sessionToken: 'session',
      masterToken: 'master',
      sessionTokenExpiresAtMs: 2_000,
      masterTokenExpiresAtMs: 3_000,
    });
  });

  it('should restore both tokens from a payload without expiration times', () => {
    expect(
      deserializeTokenInfo(
        JSON.stringify({
          services: { sf: { tokenInfo: { sessionToken: 'session', masterToken: 'master' } } },
        }),
      ),
    ).toEqual({
      sessionToken: 'session',
      masterToken: 'master',
      sessionTokenExpiresAtMs: undefined,
      masterTokenExpiresAtMs: undefined,
    });
  });

  it.each([
    ['a missing payload', undefined],
    ['a payload that is not a string', {}],
    ['a string that is not JSON', 'not json at all'],
    ['JSON null', 'null'],
    ['a JSON number', '123'],
    ['a JSON array', '[]'],
    ['an empty JSON object', '{}'],
    ['a payload whose tokenInfo is missing', JSON.stringify({ services: { sf: {} } })],
    [
      'a payload whose tokenInfo is null',
      JSON.stringify({ services: { sf: { tokenInfo: null } } }),
    ],
    [
      'a payload whose tokenInfo is a string',
      JSON.stringify({ services: { sf: { tokenInfo: 'x' } } }),
    ],
    [
      'a payload whose tokenInfo is a number',
      JSON.stringify({ services: { sf: { tokenInfo: 5 } } }),
    ],
    [
      'a payload whose tokenInfo is an array',
      JSON.stringify({ services: { sf: { tokenInfo: [] } } }),
    ],
    ['a payload whose tokenInfo is empty', serializeTokenInfo(null)],
    [
      'a payload with only a session token',
      JSON.stringify({ services: { sf: { tokenInfo: { sessionToken: 'session' } } } }),
    ],
    [
      'a payload with only a master token',
      JSON.stringify({ services: { sf: { tokenInfo: { masterToken: 'master' } } } }),
    ],
    [
      'a payload with empty tokens',
      JSON.stringify({ services: { sf: { tokenInfo: { sessionToken: '', masterToken: '' } } } }),
    ],
    [
      'a payload with tokens that are not strings',
      JSON.stringify({ services: { sf: { tokenInfo: { sessionToken: 1, masterToken: 2 } } } }),
    ],
    [
      'a payload with an expiration time that is not a number',
      JSON.stringify({
        services: {
          sf: {
            tokenInfo: {
              sessionToken: 'session',
              masterToken: 'master',
              sessionTokenExpirationTime: '2000',
            },
          },
        },
      }),
    ],
  ])('should reject %s', (_description, payload) => {
    expect(() => deserializeTokenInfo(payload as string)).toThrow(
      expect.objectContaining({
        name: 'InvalidParameterError',
        code: 408003,
        message: ErrorMessage[408003],
      }),
    );
  });
});
