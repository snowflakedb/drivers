export type LogLevel = 'ERROR' | 'WARN' | 'INFO' | 'DEBUG' | 'TRACE' | 'OFF';

/** sf_core folds TRACE into DEBUG, so both map to the same value. */
export const LOG_LEVEL_TO_CORE_VALUE: Record<LogLevel, number> = {
  OFF: -1,
  ERROR: 0,
  WARN: 1,
  INFO: 2,
  DEBUG: 3,
  TRACE: 3,
};
