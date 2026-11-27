import { logger } from './logging/index.js';

export interface SnowflakeError extends Error {
  code?: string | number;
  sqlState?: string;
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

export function invalidParameterError(message: string, code?: number): SnowflakeError {
  return Object.assign(new Error(message), {
    name: 'InvalidParameterError',
    code,
  });
}
