import fs from 'node:fs';
import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import type { Connection } from '../../../types/sdk-types.js';
import { createLiveConnection, createTempDir, createTemporaryStage } from '../../utils/fixtures.js';
import { destroyConnectionAsync, executeAsync } from '../../utils/index.js';

describe('PUT GET SSE stage', () => {
  let connection: Connection;

  beforeAll(async () => {
    connection = await createLiveConnection({}, false);
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  describe('tests/definitions/shared/put_get/put_get_sse_stage.feature', () => {
    it('should put and get file on SSE stage', async () => {
      // Given Stage with server-side encryption (SNOWFLAKE_SSE)
      const stage = await createTemporaryStage(connection, { encryption: 'SNOWFLAKE_SSE' });
      const uploadDir = createTempDir();
      const downloadDir = createTempDir();
      uploadDir.writeFile('sse_test.txt', 'hello sse\n');

      // When File is uploaded using PUT command
      const { rows: uploaded } = await executeAsync(
        connection,
        `PUT ${uploadDir.fileUrl('sse_test.txt')} @${stage} AUTO_COMPRESS=FALSE OVERWRITE=TRUE`,
      );

      // Then File should be uploaded successfully
      expect(uploaded[0].status).toBe('UPLOADED');

      // When File is downloaded using GET command
      const { rows: downloaded } = await executeAsync(
        connection,
        `GET @${stage}/sse_test.txt ${downloadDir.fileUrl()}/`,
      );

      // Then File should be downloaded
      expect(downloaded[0].status).toBe('DOWNLOADED');

      // And Have correct content
      const downloadedFile = downloadDir.resolve('sse_test.txt');
      expect(fs.existsSync(downloadedFile)).toBe(true);
      expect(fs.readFileSync(downloadedFile, 'utf8').trim()).toBe('hello sse');
    });

    it('should put and get file on SSE stage with DIRECTORY enabled', async () => {
      // Given Stage with server-side encryption and DIRECTORY enabled
      const stage = await createTemporaryStage(connection, {
        encryption: 'SNOWFLAKE_SSE',
        directory: true,
      });
      const uploadDir = createTempDir();
      const downloadDir = createTempDir();
      uploadDir.writeFile('test.txt', 'Initial contents\n');

      // When File is uploaded using PUT command
      const { rows: uploaded } = await executeAsync(
        connection,
        `PUT ${uploadDir.fileUrl('test.txt')} @${stage} AUTO_COMPRESS=FALSE OVERWRITE=TRUE`,
      );

      // Then File should be uploaded successfully
      expect(uploaded[0].status).toBe('UPLOADED');

      // When File is downloaded using GET command
      const { rows: downloaded } = await executeAsync(
        connection,
        `GET @${stage}/test.txt ${downloadDir.fileUrl()}/`,
      );

      // Then File should be downloaded
      expect(downloaded[0].status).toBe('DOWNLOADED');

      // And Have correct content
      const downloadedFile = downloadDir.resolve('test.txt');
      expect(fs.existsSync(downloadedFile)).toBe(true);
      expect(fs.readFileSync(downloadedFile, 'utf8').trim()).toBe('Initial contents');
    });
  });
});
