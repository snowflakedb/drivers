import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { afterEach, describe, expect, it, onTestFinished } from 'vitest';
import { resetGlobalConfig, updateGlobalConfig } from '../../../src/global-config.js';
import { formatLogLine } from '../../../src/logging/default-logger.js';
import { logger } from '../../../src/logging/logger.js';
import { createConsoleCapture } from '../../utils/fixtures.js';

function toMessages(writes: string[]): string[] {
  return writes.map((line) =>
    (JSON.parse(line) as { message: string }).message.replace(/^\[.*?\]: /, ''),
  );
}

describe('Logger', () => {
  afterEach(() => {
    resetGlobalConfig();
  });

  it('should loop records back to the default logger when the native bridge is not attached', () => {
    const writes = createConsoleCapture();
    updateGlobalConfig({ logFilePath: 'STDOUT' });

    logger.info('value %j', { a: 1 });

    expect(toMessages(writes)).toContain('value {"a":1}');
  });

  it('should write INFO to stdout and snowflake.log in the working directory by default', () => {
    const cwd = process.cwd();
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'sf-log-default-'));
    process.chdir(dir);
    onTestFinished(() => {
      process.chdir(cwd);
      fs.rmSync(dir, { recursive: true, force: true });
    });
    const writes = createConsoleCapture();

    logger.info('wrapper info');
    logger.debug('wrapper debug');

    expect(toMessages(writes)).toEqual(['wrapper info']);
    expect(fs.readFileSync(path.join(dir, 'snowflake.log'), 'utf8')).toMatch(/wrapper info"}\n$/);
  });
});

describe('formatLogLine', () => {
  it('should emit a JSON line with an uppercase level and a moment-style timestamp', () => {
    const now = new Date(2026, 0, 1, 15, 4, 5, 6);
    expect(formatLogLine('INFO', 'hello', now)).toBe(
      `${JSON.stringify({ level: 'INFO', message: '[3:04:05.006 PM]: hello' })}\n`,
    );
  });

  it('should format midnight and noon on the 12-hour clock', () => {
    const line = (hour: number) =>
      (
        JSON.parse(formatLogLine('INFO', 'x', new Date(2026, 0, 1, hour))) as {
          message: string;
        }
      ).message;
    expect(line(0)).toBe('[12:00:00.000 AM]: x');
    expect(line(12)).toBe('[12:00:00.000 PM]: x');
  });

  it('should truncate messages to 500 characters', () => {
    const parsed = JSON.parse(formatLogLine('INFO', 'a'.repeat(600))) as {
      message: string;
    };
    expect(parsed.message.replace(/^\[.*?\]: /, '')).toHaveLength(500);
  });
});
