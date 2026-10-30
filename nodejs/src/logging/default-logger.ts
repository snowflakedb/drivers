import fs from 'node:fs';
import path from 'node:path';
import type { SnowflakeLogger } from './logger.js';
import { GlobalConfig } from '../global-config.js';

type LevelName = 'ERROR' | 'WARN' | 'INFO' | 'DEBUG';

const STDOUT_PATH = 'STDOUT';
const MESSAGE_MAX_LENGTH = 500;

class LogFile {
  #descriptor?: number;
  #failed = false;

  constructor(readonly path: string) {}

  open(): number {
    if (this.#descriptor === undefined) {
      fs.mkdirSync(path.dirname(this.path), { recursive: true });
      this.#descriptor = fs.openSync(this.path, 'a');
    }
    return this.#descriptor;
  }

  write(line: string): void {
    if (this.#failed) {
      return;
    }
    try {
      fs.writeSync(this.open(), line);
    } catch (error) {
      this.#failed = true;
      consoleStdout().write(
        formatLogLine('ERROR', `cannot write log file ${this.path}: ${String(error)}`),
      );
    }
  }

  close(): void {
    if (this.#descriptor !== undefined) {
      fs.closeSync(this.#descriptor);
      this.#descriptor = undefined;
    }
  }
}

/**
 * Writes JSON log lines to `GlobalConfig.logFilePath`, and also to stdout unless
 * `additionalLogToConsole` is `false`. With `logFilePath` set to `STDOUT`, it writes to stdout only.
 */
class DefaultLogger implements SnowflakeLogger {
  #file?: LogFile;

  error(message: string): void {
    this.#write('ERROR', message);
  }

  warn(message: string): void {
    this.#write('WARN', message);
  }

  info(message: string): void {
    this.#write('INFO', message);
  }

  debug(message: string): void {
    this.#write('DEBUG', message);
  }

  trace(message: string): void {
    this.#write('DEBUG', message);
  }

  /** Throws when the configured file cannot be opened. */
  openFile(): void {
    this.#syncFile()?.open();
  }

  close(): void {
    this.#file?.close();
    this.#file = undefined;
  }

  /** A `logFilePath` of `STDOUT` (case-insensitive) means no file, stdout only. */
  #syncFile(): LogFile | undefined {
    const { logFilePath } = GlobalConfig;
    if (logFilePath.toUpperCase() === STDOUT_PATH) {
      this.close();
    } else if (this.#file?.path !== logFilePath) {
      this.close();
      this.#file = new LogFile(logFilePath);
    }
    return this.#file;
  }

  #write(level: LevelName, message: string): void {
    const line = formatLogLine(level, message);
    const file = this.#syncFile();
    if (!file || GlobalConfig.additionalLogToConsole !== false) {
      consoleStdout().write(line);
    }
    file?.write(line);
  }
}

export const defaultLogger = new DefaultLogger();

/**
 * The stream behind the global `console`. Console log lines go here, not to `process.stdout`.
 * - In plain Node.js it is `process.stdout` itself, so writes cost the same.
 * - Test runners such as Vitest replace the global `console`, and their output settings
 *   (for example `silent: 'passed-only'`) apply only to that console. Writes to
 *   `process.stdout` skip them and always print.
 * - It is read on every write because a test runner can replace `console` after this module loads.
 * - `console.log` is not used: it is slower, and some hosts change what it prints. For example,
 *   AWS Lambda currently adds a timestamp and request ID to each `console.log` line.
 * - The 3.x driver writes to the same stream, through winston's console transport.
 *
 * `_stdout` is not documented by Node.js, so this falls back to `process.stdout` when it is missing.
 */
function consoleStdout(): NodeJS.WritableStream {
  return (console as unknown as { _stdout?: NodeJS.WritableStream })._stdout ?? process.stdout;
}

export function formatLogLine(level: LevelName, message: string, now = new Date()): string {
  const truncated = message.slice(0, MESSAGE_MAX_LENGTH);
  return `${JSON.stringify({ level, message: `[${formatTimestamp(now)}]: ${truncated}` })}\n`;
}

function pad(value: number, width = 2): string {
  return String(value).padStart(width, '0');
}

// TODO: consider replacing this with date.toISOString, current `h:mm:ss.SSS A` matches the old driver's log lines
function formatTimestamp(date: Date): string {
  const hours = date.getHours();
  return `${hours % 12 || 12}:${pad(date.getMinutes())}:${pad(date.getSeconds())}.${pad(date.getMilliseconds(), 3)} ${hours >= 12 ? 'PM' : 'AM'}`;
}
