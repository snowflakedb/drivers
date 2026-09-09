import os from 'node:os';
import path from 'node:path';
import { describe, it, expect, beforeAll, afterAll, beforeEach, afterEach } from 'vitest';
import type { Connection } from '../../../types/sdk-types.js';
import { toFileUrl } from '../../utils/files.js';
import { createLiveConnection, createTempDir } from '../../utils/fixtures.js';
import {
  destroyConnectionAsync,
  executeAsync,
  isRunningNewDriverWithBD,
  randomizeName,
} from '../../utils/index.js';

describe('PUT source path normalization', () => {
  let connection: Connection;

  beforeAll(async () => {
    connection = await createLiveConnection({}, false);
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  describe('tests/definitions/shared/put_get/put_get_path_normalization.feature', () => {
    let stage: string;

    beforeEach(async () => {
      stage = randomizeName('TEST_PUT_PATH_NORM');
      await executeAsync(connection, `CREATE TEMPORARY STAGE IF NOT EXISTS ${stage}`);
    });

    afterEach(async () => {
      await executeAsync(connection, `DROP STAGE IF EXISTS ${stage}`);
    });

    it('should upload file when source path contains dotdot segments', async () => {
      // Given A source file exists in a temporary directory
      const tempDir = createTempDir();
      const subDir = tempDir.mkdir('sub');
      tempDir.writeFile('dotdot_data.csv');

      // When PUT command is executed with a source path containing dotdot segments
      const dotdotPath = path.join(subDir, '..', 'dotdot_data.csv');
      const { rows } = await executeAsync(
        connection,
        `PUT ${toFileUrl(dotdotPath)} @${stage} AUTO_COMPRESS=FALSE OVERWRITE=TRUE`,
      );

      // Then File is uploaded successfully with correct target name
      expect(rows[0].status).toBe('UPLOADED');
      expect(rows[0].target).toBe('dotdot_data.csv');
    });

    it('should upload file when source path is relative to working directory', async () => {
      // Create the workdir under CWD so the relative path stays same-drive on Windows.
      const workDir = createTempDir({ root: process.cwd() });
      // Given A source file exists in a temporary directory
      workDir.writeFile('relative_data.csv');

      // When PUT command is executed with a path relative to the process working directory
      const { rows } = await executeAsync(
        connection,
        `PUT file://${workDir.relative(process.cwd(), 'relative_data.csv')} @${stage} AUTO_COMPRESS=FALSE OVERWRITE=TRUE`,
      );

      // Then File is uploaded successfully with correct target name
      expect(rows[0].status).toBe('UPLOADED');
      expect(rows[0].target).toBe('relative_data.csv');
    });

    // TODO: mark in feature file when step_finder.rs is fixed by
    // https://github.com/snowflake-eng/drivers/pull/1764
    it.skipIf(process.platform === 'win32')(
      'should upload file at symlinked source path',
      async () => {
        // Given A source file and a symlink pointing to it exist in a temporary directory
        const tempDir = createTempDir();
        tempDir.writeFile('real.csv');
        tempDir.linkFile('real.csv', 'link.csv');

        // When PUT command is executed with the symlink as source path
        const { rows } = await executeAsync(
          connection,
          `PUT ${tempDir.fileUrl('link.csv')} @${stage} AUTO_COMPRESS=FALSE OVERWRITE=TRUE`,
        );

        // Then File is uploaded successfully
        expect(rows[0].status).toBe('UPLOADED');
        expect(rows[0].target).toBe(isRunningNewDriverWithBD('BD#30') ? 'real.csv' : 'link.csv');
      },
    );

    it('should upload file when source path starts with tilde', async () => {
      // Given A source file exists in a subdirectory under the home directory
      const subDir = createTempDir({ root: os.homedir() });
      subDir.writeFile('tilde_data.csv');

      // When PUT command is executed with a leading ~ in the source path
      const putResult = executeAsync(
        connection,
        `PUT file://~/${subDir.name}/tilde_data.csv @${stage} AUTO_COMPRESS=FALSE OVERWRITE=TRUE`,
      ).then(
        ({ rows }) => ({ rows, error: null }),
        (thrown: { error: unknown }) => ({ rows: null, error: thrown.error }),
      );
      const { rows, error } = await putResult;

      // Then File is uploaded successfully
      if (isRunningNewDriverWithBD('BD#31')) {
        expect(rows![0].status).toBe('UPLOADED');
      } else {
        expect(String(error)).toContain('not a directory');
      }
    });
  });
});
