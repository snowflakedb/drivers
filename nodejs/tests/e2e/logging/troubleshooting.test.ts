import { execFile } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';
import { describe, expect, it } from 'vitest';
import { createTempDir } from '../utils/fixtures.js';
import { IS_RUNNING_FOR_OLD_DRIVER, TEST_CONNECTION_OPTIONS } from '../utils/index.js';

const NODEJS_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../..');
const WRAPPER_DEBUG_MARKER = 'wrapper troubleshooting debug line';

// Troubleshooting mode is read once when the native module loads, so the driver runs in a
// child process that starts with the environment variables already set.
const WORKER_CODE = `
import snowflake from 'snowflake-sdk';
import { logger } from 'snowflake-sdk/dist/logging/logger.js';
const connection = snowflake.createConnection(JSON.parse(process.env.SF_TEST_CONNECTION_OPTIONS));
await connection.connectAsync();
logger.debug('${WRAPPER_DEBUG_MARKER}');
await new Promise((resolve, reject) =>
  connection.execute({ sqlText: 'SELECT 1', complete: (err) => (err ? reject(err) : resolve()) }),
);
await new Promise((resolve) => connection.destroy(resolve));
`;

describe.skipIf(IS_RUNNING_FOR_OLD_DRIVER)('troubleshooting mode', () => {
  describe('tests/definitions/shared/logging/troubleshooting.feature', () => {
    it('should create troubleshooting log file when enabled via environment variable', async () => {
      // Given SNOWFLAKE_TROUBLESHOOTING_ENABLED is set to "true" and SNOWFLAKE_TROUBLESHOOTING_REPORT_PATH points to a temporary directory
      const logDir = createTempDir().path;
      const env = {
        ...process.env,
        SNOWFLAKE_TROUBLESHOOTING_ENABLED: 'true',
        SNOWFLAKE_TROUBLESHOOTING_REPORT_PATH: logDir,
        SF_TEST_CONNECTION_OPTIONS: JSON.stringify(TEST_CONNECTION_OPTIONS),
      };

      // When a connection is established and a query is executed
      await promisify(execFile)(process.execPath, ['--input-type=module', '-e', WORKER_CODE], {
        cwd: NODEJS_ROOT,
        env,
        timeout: 120_000,
      });

      // Then a troubleshooting log file exists in the configured directory
      const logFile = path.join(logDir, 'sf_driver_troubleshooting.log');
      expect(fs.existsSync(logFile)).toBe(true);

      // And the log file contains debug-level entries below the configured log level
      expect(fs.readFileSync(logFile, 'utf8')).toContain(WRAPPER_DEBUG_MARKER);
    });
  });
});
