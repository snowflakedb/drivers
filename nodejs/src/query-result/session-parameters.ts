import type { CoreConnectionInstance } from '../core/index.js';
import type { SessionParameters } from './types.js';
import SessionParameterName from '../constants/SessionParameterName.js';

function isEnabled(connection: CoreConnectionInstance, name: string): boolean {
  const parameter = connection.getSessionParameter(name);
  if (parameter === null) {
    return false;
  }
  return parameter.getBool() ?? parameter.getString()?.toLowerCase() === 'true';
}

export function readSessionParameters(connection: CoreConnectionInstance): SessionParameters {
  return {
    treatIntegerAsBigInt: isEnabled(connection, SessionParameterName.JS_TREAT_INTEGER_AS_BIGINT),
  };
}
