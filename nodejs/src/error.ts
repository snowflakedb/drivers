import { logger } from './logging/index.js';

export interface SnowflakeError extends Error {
  code?: string | number;
  sqlState?: string;
  /**
   * `true` when the connection can't be used anymore, so you have to create a new one.
   * Every later call on the connection fails, and `isUp()` returns `false`.
   *
   * The driver sets it when:
   * - `connect()` fails, or is called on a connection that can't be used anymore.
   * - Snowflake ends the session, for example when the master token expires.
   * - you use a connection that was destroyed or whose login failed.
   */
  isFatal?: boolean;
  /**
   * Extra details about the HTTP response, such as the status code.
   * Set only when `name` is `'RequestFailedError'`.
   *
   * It has no response body, because the body can contain secrets that could end up in logs.
   * To see the full response, send the traffic through a proxy and inspect it there.
   */
  response?: { statusCode: number; statusText: string };
}

interface ErrorReporter {
  sendErrorTelemetryEvent(exceptionType: string, errorSource: string): void;
}

export function tryWithErrorReporter<T>(
  reporter: ErrorReporter,
  errorSource: string,
  fn: () => T,
): T {
  try {
    return fn();
  } catch (err) {
    const exceptionType = err instanceof Error ? err.name : typeof err;
    try {
      reporter.sendErrorTelemetryEvent(exceptionType, errorSource);
    } catch (reportErr) {
      logger.debug(
        `Failed to send ${exceptionType} error telemetry: ${reportErr instanceof Error ? reportErr.name : typeof reportErr}`,
      );
    }
    throw err;
  }
}

export function invalidParameterError(message: string): SnowflakeError {
  return Object.assign(new Error(message), {
    name: 'InvalidParameterError',
    // Every driver error has a `code` key, even when it has no code, as in 3.x.
    code: undefined,
  });
}
