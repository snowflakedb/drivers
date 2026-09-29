import { describe, expect, it } from 'vitest';
import { serializeTokenInfo } from '../../src/connection-serialization.js';

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
});
