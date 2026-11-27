import type { ConnectionTokenInfo } from './core/binary-types.generated.js';
import { invalidParameterError } from './error.js';

export function serializeTokenInfo(info: ConnectionTokenInfo | null): string {
  return JSON.stringify({
    services: {
      sf: {
        tokenInfo: info
          ? {
              sessionToken: info.sessionToken,
              masterToken: info.masterToken,
              sessionTokenExpirationTime: info.sessionTokenExpiresAtMs ?? undefined,
              masterTokenExpirationTime: info.masterTokenExpiresAtMs ?? undefined,
            }
          : {},
      },
    },
  });
}

// TODO: a follow-up PR will remove expiration times from ConnectionTokenInfo
export function deserializeTokenInfo(serializedConnection: string): ConnectionTokenInfo {
  let sessionToken: unknown;
  let masterToken: unknown;
  let sessionTokenExpirationTime: unknown;
  let masterTokenExpirationTime: unknown;

  try {
    ({ sessionToken, masterToken, sessionTokenExpirationTime, masterTokenExpirationTime } =
      JSON.parse(serializedConnection).services.sf.tokenInfo);
  } catch {
    throwDeserializeError();
  }

  if (
    !isNonEmptyString(sessionToken) ||
    !isNonEmptyString(masterToken) ||
    !isOptionalNumber(sessionTokenExpirationTime) ||
    !isOptionalNumber(masterTokenExpirationTime)
  ) {
    throwDeserializeError();
  }

  return {
    sessionToken,
    masterToken,
    sessionTokenExpiresAtMs: sessionTokenExpirationTime,
    masterTokenExpiresAtMs: masterTokenExpirationTime,
  };
}

function isNonEmptyString(value: unknown): value is string {
  return typeof value === 'string' && value.length > 0;
}

function isOptionalNumber(value: unknown): value is number | undefined {
  return value === undefined || typeof value === 'number';
}

function throwDeserializeError(): never {
  throw invalidParameterError(
    "Invalid serializedConnection. The value must be a string obtained by calling another connection's serialize() method.",
  );
}
