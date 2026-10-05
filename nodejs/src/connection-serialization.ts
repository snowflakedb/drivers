import type { ConnectionTokenInfo } from './core/binary-types.generated.js';
import type { SnowflakeError } from './error.js';
import ErrorCode from './constants/ErrorCode.js';
import ErrorMessage from './constants/ErrorMessage.js';

export type DeserializedTokenInfo = {
  sessionToken?: string;
  masterToken?: string;
};

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

export function deserializeTokenInfo(serializedConnection: unknown): DeserializedTokenInfo {
  if (serializedConnection === undefined || serializedConnection === null) {
    throwDeserializeError(ErrorCode.ERR_CONN_DESERIALIZE_MISSING_CONFIG);
  }
  if (typeof serializedConnection !== 'string') {
    throwDeserializeError(ErrorCode.ERR_CONN_DESERIALIZE_INVALID_CONFIG_TYPE);
  }

  let parsed: unknown;
  try {
    parsed = JSON.parse(serializedConnection);
  } catch {
    throwDeserializeError(ErrorCode.ERR_CONN_DESERIALIZE_INVALID_CONFIG_FORM);
  }

  if (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed)) {
    throwDeserializeError(ErrorCode.ERR_CONN_DESERIALIZE_INVALID_CONFIG_FORM);
  }

  const tokenInfo = (parsed as { services?: { sf?: { tokenInfo?: unknown } } }).services?.sf
    ?.tokenInfo;
  if (typeof tokenInfo !== 'object' || tokenInfo === null || Array.isArray(tokenInfo)) {
    throwDeserializeError(ErrorCode.ERR_CONN_DESERIALIZE_INVALID_CONFIG_FORM);
  }

  const sessionToken = (tokenInfo as { sessionToken?: unknown }).sessionToken;
  const masterToken = (tokenInfo as { masterToken?: unknown }).masterToken;
  const hasSession = typeof sessionToken === 'string' && sessionToken.length > 0;
  const hasMaster = typeof masterToken === 'string' && masterToken.length > 0;
  if (hasSession !== hasMaster) {
    throwDeserializeError(ErrorCode.ERR_CONN_DESERIALIZE_INVALID_CONFIG_FORM);
  }
  if (typeof sessionToken === 'string' && typeof masterToken === 'string' && hasSession) {
    return { sessionToken, masterToken };
  }
  return {};
}

function throwDeserializeError(code: number): never {
  const error = new Error(ErrorMessage[code]) as SnowflakeError;
  error.name = 'InvalidParameterError';
  error.code = code;
  throw error;
}
