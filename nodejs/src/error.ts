import { logger } from './logging/index.js';

export interface SnowflakeError extends Error {
  code?: string | number;
  sqlState?: string;
  isFatal?: boolean;
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
