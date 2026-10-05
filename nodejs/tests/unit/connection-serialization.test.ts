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

  it('should serialize empty token info as an empty object', () => {
    expect(serializeTokenInfo({})).toBe(JSON.stringify({ services: { sf: { tokenInfo: {} } } }));
  });

  it('should restore both tokens from a serialize payload', () => {
    expect(
      deserializeTokenInfo(
        serializeTokenInfo({
          sessionToken: 'session',
          masterToken: 'master',
          sessionTokenExpiresAtMs: 2_000,
          masterTokenExpiresAtMs: 3_000,
        }),
      ),
    ).toEqual({ sessionToken: 'session', masterToken: 'master' });
  });

  it('should treat an empty tokenInfo object as disconnected', () => {
    expect(deserializeTokenInfo(serializeTokenInfo({}))).toEqual({});
  });

  it('should reject a missing payload', () => {
    expect(() => deserializeTokenInfo(undefined)).toThrow(
      expect.objectContaining({
        name: 'InvalidParameterError',
        code: 408001,
        message: ErrorMessage[408001],
      }),
    );
  });

  it('should reject a non-string payload', () => {
    expect(() => deserializeTokenInfo({})).toThrow(
      expect.objectContaining({
        name: 'InvalidParameterError',
        code: 408002,
        message: ErrorMessage[408002],
      }),
    );
  });

  it.each([
    ['a string that is not JSON', 'not json at all'],
    ['a JSON number', '123'],
    ['a JSON array', '[]'],
    ['an empty JSON object', '{}'],
    ['a payload whose tokenInfo is missing', JSON.stringify({ services: { sf: {} } })],
    [
      'a payload with only a session token',
      JSON.stringify({ services: { sf: { tokenInfo: { sessionToken: 'session' } } } }),
    ],
  ])('should reject %s', (_description, payload) => {
    expect(() => deserializeTokenInfo(payload)).toThrow(
      expect.objectContaining({
        name: 'InvalidParameterError',
        code: 408003,
        message: ErrorMessage[408003],
      }),
    );
  });
});
