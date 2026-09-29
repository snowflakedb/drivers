import type { ConnectionTokenInfo } from './core/binary-types.generated.js';

export function serializeTokenInfo(info: ConnectionTokenInfo): string {
  return JSON.stringify({
    services: {
      sf: {
        tokenInfo: {
          sessionToken: info.sessionToken,
          masterToken: info.masterToken,
          sessionTokenExpirationTime: info.sessionTokenExpiresAtMs ?? undefined,
          masterTokenExpirationTime: info.masterTokenExpiresAtMs ?? undefined,
        },
      },
    },
  });
}
