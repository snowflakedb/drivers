import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { afterEach, describe, expect, it, onTestFinished, vi } from 'vitest';
import { resetGlobalConfig, updateGlobalConfig } from '../../../src/global-config.js';
import { logger } from '../../../src/logging/logger.js';
import { createConsoleCapture } from '../../utils/fixtures.js';

function createTempLogPath(): string {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'sf-log-'));
  onTestFinished(() => fs.rmSync(dir, { recursive: true, force: true }));
  return path.join(dir, 'snowflake.log');
}

function getLogLines(filePath: string): { level: string; message: string }[] {
  return fs
    .readFileSync(filePath, 'utf8')
    .split('\n')
    .filter((line) => line.length > 0)
    .map((line) => JSON.parse(line) as { level: string; message: string });
}

describe('configure logging', () => {
  afterEach(() => {
    resetGlobalConfig();
  });

  it('should treat TRACE as DEBUG', () => {
    const filePath = createTempLogPath();
    updateGlobalConfig({
      logLevel: 'TRACE',
      logFilePath: filePath,
      additionalLogToConsole: false,
    });

    logger.debug('debug from trace');

    expect(getLogLines(filePath).map((line) => line.level)).toContain('DEBUG');
  });

  it('should write nothing when logLevel is OFF', () => {
    const filePath = createTempLogPath();
    updateGlobalConfig({
      logLevel: 'OFF',
      logFilePath: filePath,
      additionalLogToConsole: false,
    });

    logger.error('suppressed');

    expect(fs.existsSync(filePath)).toBe(false);
  });

  it('should keep earlier logging options when a later configure call sets only one', () => {
    const stdout = createConsoleCapture();
    const filePath = createTempLogPath();
    updateGlobalConfig({
      logLevel: 'WARN',
      logFilePath: filePath,
      additionalLogToConsole: false,
    });

    updateGlobalConfig({ additionalLogToConsole: true });
    logger.info('below level');
    logger.warn('to both');

    expect(stdout.at(-1)).toMatch(/to both"}\n$/);
    expect(getLogLines(filePath).map((line) => line.message)).toEqual([
      expect.stringMatching(/to both$/),
    ]);
  });

  it('should switch files when logFilePath is configured again', () => {
    const first = createTempLogPath();
    const second = createTempLogPath();
    updateGlobalConfig({ logFilePath: first, additionalLogToConsole: false });
    logger.info('first file');

    updateGlobalConfig({ logFilePath: second, additionalLogToConsole: false });
    logger.info('second file');

    expect(getLogLines(first).map((line) => line.message)).not.toContainEqual(
      expect.stringMatching(/second file$/),
    );
    expect(getLogLines(second).at(-1)?.message).toMatch(/second file$/);
  });

  it('should throw from configure when logFilePath cannot be opened', () => {
    const regularFile = createTempLogPath();
    fs.writeFileSync(regularFile, '');

    expect(() =>
      updateGlobalConfig({
        logFilePath: path.join(regularFile, 'snowflake.log'),
        additionalLogToConsole: false,
      }),
    ).toThrow(/ENOTDIR|EEXIST/);
  });

  it('should not throw from log calls when the default log file cannot be opened', () => {
    const stdout = createConsoleCapture();
    const openSync = vi.spyOn(fs, 'openSync').mockImplementation(() => {
      throw new Error('EACCES: permission denied');
    });
    onTestFinished(() => openSync.mockRestore());

    expect(() => logger.error('no file')).not.toThrow();
    expect(stdout.map((line) => JSON.parse(line))).toContainEqual({
      level: 'ERROR',
      message: expect.stringMatching(/cannot write log file snowflake\.log/),
    });
  });

  it('should keep customLogger until it is set back to undefined', () => {
    const stdout = createConsoleCapture();
    const customLogger = {
      error: vi.fn(),
      warn: vi.fn(),
      info: vi.fn(),
      debug: vi.fn(),
      trace: vi.fn(),
    };
    updateGlobalConfig({ customLogger });

    updateGlobalConfig({ logFilePath: 'STDOUT' });
    logger.info('still custom');
    updateGlobalConfig({ customLogger: undefined });
    logger.info('built-in again');

    expect(customLogger.info).toHaveBeenCalledWith('still custom');

    expect(customLogger.info).not.toHaveBeenCalledWith('built-in again');
    expect(stdout.at(-1)).toMatch(/built-in again"}\n$/);
  });
});
