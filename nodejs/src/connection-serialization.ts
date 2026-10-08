import type { ConnectionTokenInfo } from './core/binary-types.generated.js';
import type { SnowflakeError } from './error.js';
import ErrorCode from './constants/ErrorCode.js';
import ErrorMessage from './constants/ErrorMessage.js';

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
  // TODO: We don't want InvalidParameterError to have error codes.
  const code = ErrorCode.ERR_CONN_DESERIALIZE_INVALID_CONFIG_FORM;
  const error = new Error(ErrorMessage[code]) as SnowflakeError;
  error.name = 'InvalidParameterError';
  error.code = code;
  throw error;
}
