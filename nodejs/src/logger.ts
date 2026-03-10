import { initNativeLogger } from "./sf_core_client/transport";

export type LogLevel = "OFF" | "ERROR" | "WARN" | "INFO" | "DEBUG" | "TRACE";

export interface ConfigureOptions {
  logLevel?: LogLevel;
}

// sf_core numeric levels -> our level names
const SF_CORE_LEVEL_MAP: Record<number, LogLevel> = {
  0: "ERROR",
  1: "WARN",
  2: "INFO",
  3: "DEBUG",
};

const LEVEL_PRIORITY: Record<LogLevel, number> = {
  OFF: -1,
  ERROR: 0,
  WARN: 1,
  INFO: 2,
  DEBUG: 3,
  TRACE: 4,
};

const LOG_FN: Record<LogLevel, (...args: unknown[]) => void> = {
  OFF: () => {},
  ERROR: console.error,
  WARN: console.warn,
  INFO: console.info,
  DEBUG: console.debug,
  TRACE: console.debug,
};

let currentLevel: LogLevel = "OFF";
let nativeLoggerInitialized = false;

export function configure(options: ConfigureOptions): void {
  if (options.logLevel !== undefined) {
    currentLevel = options.logLevel;
  }

  if (!nativeLoggerInitialized && currentLevel !== "OFF") {
    nativeLoggerInitialized = true;
    initNativeLogger((level, message, filename, line, func) => {
      const levelName = SF_CORE_LEVEL_MAP[level] ?? "DEBUG";
      if (LEVEL_PRIORITY[levelName] > LEVEL_PRIORITY[currentLevel]) {
        return;
      }
      const logFn = LOG_FN[levelName];
      logFn(`[${levelName}] [${filename}:${line} ${func}] ${message}`);
    });
  }
}
