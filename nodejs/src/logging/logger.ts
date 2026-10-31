import { format } from 'node:util';
import type { CoreLogEvent } from '../core/index.js';
import { GlobalConfig } from '../global-config.js';
import { defaultLogger } from './default-logger.js';
import { LOG_LEVEL_TO_CORE_VALUE } from './log-level.js';

const { ERROR, WARN, INFO, DEBUG } = LOG_LEVEL_TO_CORE_VALUE;

export interface SnowflakeLogger {
  error(message: string): void;
  warn(message: string): void;
  info(message: string): void;
  debug(message: string): void;
  /**
   * The driver doesn't produce trace-level logs, but this method is kept for backward
   * compatibility with older driver versions.
   */
  trace(message: string): void;
}

export interface CoreLogging {
  logEvent(level: number, message: string): void;
  setLogLevel(level: number): void;
}

/**
 * With the core attached, wrapper records go through sf_core first so every core layer
 * (troubleshooting file, etc.) sees them, and come back through `handleCoreLogEvent`
 * already filtered by the level the core holds. Without it, they are handled directly.
 */
class LogRouter {
  #core?: CoreLogging;

  setup(core?: CoreLogging): void {
    this.#core = core;
    this.syncLevel();
  }

  syncLevel(): void {
    this.#core?.setLogLevel(this.#configuredLevel());
  }

  log(level: number, msg: string, args: unknown[]): void {
    if (level > this.#configuredLevel()) {
      return;
    }
    const message = format(msg, ...args);
    if (this.#core) {
      this.#core.logEvent(level, message);
    } else {
      this.handleCoreLogEvent({ level, message });
    }
  }

  handleCoreLogEvent = ({ level, message }: CoreLogEvent): void => {
    const target = GlobalConfig.customLogger ?? defaultLogger;
    switch (level) {
      case ERROR:
        return target.error(message);
      case WARN:
        return target.warn(message);
      case INFO:
        return target.info(message);
      default:
        return target.debug(message);
    }
  };

  #configuredLevel(): number {
    return LOG_LEVEL_TO_CORE_VALUE[GlobalConfig.logLevel];
  }
}

export const logRouter = new LogRouter();

export const logger = {
  error: (msg: string, ...args: unknown[]) => logRouter.log(ERROR, msg, args),
  warn: (msg: string, ...args: unknown[]) => logRouter.log(WARN, msg, args),
  info: (msg: string, ...args: unknown[]) => logRouter.log(INFO, msg, args),
  debug: (msg: string, ...args: unknown[]) => logRouter.log(DEBUG, msg, args),
};
