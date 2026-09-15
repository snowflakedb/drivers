import fs from 'node:fs';
import path from 'node:path';
import zlib from 'node:zlib';
import { describe, it, expect, beforeAll, afterAll, beforeEach, afterEach } from 'vitest';
import type { Connection } from '../../../types/sdk-types.js';
import { sharedTestDataDir, toFileUrl } from '../../utils/files.js';
import { createLiveConnection, createTempDir } from '../../utils/fixtures.js';
import { destroyConnectionAsync, executeAsync, randomizeName } from '../../utils/index.js';

describe('tests/definitions/shared/put_get/put_get_auto_compress.feature', () => {
  const TEST_FILE = path.join(sharedTestDataDir(), 'compression', 'test_data.csv');

  let connection: Connection;
  let stage: string;

  beforeAll(async () => {
    connection = await createLiveConnection({}, false);
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  beforeEach(async () => {
    stage = randomizeName('TEST_PUT_GET_AUTO_COMPRESS');
    await executeAsync(connection, `CREATE TEMPORARY STAGE IF NOT EXISTS ${stage}`);
  });

  afterEach(async () => {
    await executeAsync(connection, `DROP STAGE IF EXISTS ${stage}`);
  });

  it('should compress the file before uploading to stage when AUTO_COMPRESS set to true', async () => {
    // Given Snowflake client is logged in
    void connection;

    // When File is uploaded to stage with AUTO_COMPRESS set to true
    await executeAsync(
      connection,
      `PUT ${toFileUrl(TEST_FILE)} @${stage} AUTO_COMPRESS=TRUE OVERWRITE=TRUE`,
    );

    const downloadDir = createTempDir();
    // Then Only compressed file should be downloaded
    const { rows } = await executeAsync(
      connection,
      `GET @${stage}/test_data.csv ${downloadDir.fileUrl()}/`,
    );
    expect(rows[0].status).toBe('DOWNLOADED');

    const downloaded = fs.readdirSync(downloadDir.path);
    expect(downloaded).toContain('test_data.csv.gz');
    expect(downloaded).not.toContain('test_data.csv');

    // And Have correct content
    const content = zlib.gunzipSync(fs.readFileSync(downloadDir.resolve('test_data.csv.gz')));
    expect(content.toString().trim()).toBe('1,2,3');
  });

  it('should not compress the file before uploading to stage when AUTO_COMPRESS set to false', async () => {
    // Given Snowflake client is logged in
    void connection;

    // When File is uploaded to stage with AUTO_COMPRESS set to false
    await executeAsync(
      connection,
      `PUT ${toFileUrl(TEST_FILE)} @${stage} AUTO_COMPRESS=FALSE OVERWRITE=TRUE`,
    );

    const downloadDir = createTempDir();
    // Then Only uncompressed file should be downloaded
    const { rows } = await executeAsync(
      connection,
      `GET @${stage}/test_data.csv ${downloadDir.fileUrl()}/`,
    );
    expect(rows[0].status).toBe('DOWNLOADED');

    const downloaded = fs.readdirSync(downloadDir.path);
    expect(downloaded).toContain('test_data.csv');
    expect(downloaded).not.toContain('test_data.csv.gz');

    // And Have correct content
    expect(fs.readFileSync(downloadDir.resolve('test_data.csv'), 'utf8').trim()).toBe('1,2,3');
  });
});
