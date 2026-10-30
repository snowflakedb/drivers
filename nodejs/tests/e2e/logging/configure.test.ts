import fs from 'node:fs';
import path from 'node:path';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { createConsoleCapture } from '../../utils/fixtures.js';
import { createTempDir } from '../utils/fixtures.js';
import { resetGlobalConfig, snowflake } from '../utils/index.js';

const CONFIGURE_MESSAGE = /Configuring logger with level: /;

function createTempLogPath(...segments: string[]): string {
  return path.join(createTempDir({ prefix: 'sf-log' }).path, ...segments, 'snowflake.log');
}

function getConfigureMessages(filePath: string): string[] {
  if (!fs.existsSync(filePath)) {
    return [];
  }
  return fs
    .readFileSync(filePath, 'utf8')
    .split('\n')
    .filter((line) => CONFIGURE_MESSAGE.test(line));
}

function waitForConfigureMessages(filePath: string, count: number): Promise<string[]> {
  return vi.waitUntil(
    () => {
      const messages = getConfigureMessages(filePath);
      return messages.length >= count && messages;
    },
    { timeout: 2000 },
  );
}

function waitForStdout(stdout: string[], pattern: RegExp): Promise<boolean> {
  return vi.waitUntil(() => pattern.test(stdout.join('')), { timeout: 2000 });
}

describe('configure logging', () => {
  afterEach(() => {
    resetGlobalConfig();
  });

  it('should write JSON lines to logFilePath and create missing parent directories', async () => {
    const filePath = createTempLogPath('nested', 'dir');

    snowflake.configure({
      logLevel: 'INFO',
      logFilePath: filePath,
      additionalLogToConsole: false,
    });

    const [line] = await waitForConfigureMessages(filePath, 1);
    expect(JSON.parse(line!)).toEqual({
      level: 'INFO',
      message: expect.stringMatching(
        /^\[\d{1,2}:\d{2}:\d{2}\.\d{3} (AM|PM)\]: Configuring logger with level: /,
      ),
    });
  });

  it('should not write records below logLevel', async () => {
    const filePath = createTempLogPath();

    snowflake.configure({
      logLevel: 'WARN',
      logFilePath: filePath,
      additionalLogToConsole: false,
    });
    snowflake.configure({
      logLevel: 'INFO',
      logFilePath: filePath,
      additionalLogToConsole: false,
    });

    await waitForConfigureMessages(filePath, 1);
    expect(getConfigureMessages(filePath)).toEqual([expect.stringContaining('filePath: ')]);
  });

  it('should write only to stdout when logFilePath is STDOUT', async () => {
    const stdout = createConsoleCapture();
    const cwdFiles = fs.readdirSync(process.cwd());

    snowflake.configure({ logLevel: 'INFO', logFilePath: 'STDOUT' });

    await waitForStdout(stdout, CONFIGURE_MESSAGE);
    expect(fs.readdirSync(process.cwd())).toEqual(cwdFiles);
  });

  it('should also write to stdout when additionalLogToConsole is true', async () => {
    const stdout = createConsoleCapture();
    const filePath = createTempLogPath();

    snowflake.configure({
      logLevel: 'INFO',
      logFilePath: filePath,
      additionalLogToConsole: true,
    });

    await waitForConfigureMessages(filePath, 1);
    await waitForStdout(stdout, CONFIGURE_MESSAGE);
  });

  it('should send records to customLogger instead of stdout', async () => {
    const stdout = createConsoleCapture();
    const customLogger = {
      error: vi.fn(),
      warn: vi.fn(),
      info: vi.fn(),
      debug: vi.fn(),
      trace: vi.fn(),
    };

    snowflake.configure({ logLevel: 'INFO', customLogger });

    await vi.waitUntil(() => customLogger.info.mock.calls.length > 0, { timeout: 2000 });
    expect(customLogger.info).toHaveBeenCalledWith(expect.stringMatching(CONFIGURE_MESSAGE));
    expect(stdout.join('')).not.toMatch(CONFIGURE_MESSAGE);
  });
});
