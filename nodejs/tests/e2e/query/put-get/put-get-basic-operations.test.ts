import fs from 'node:fs';
import path from 'node:path';
import zlib from 'node:zlib';
import { describe, it, expect, beforeAll, afterAll, beforeEach, afterEach } from 'vitest';
import type { Connection, RowStatement } from '../../../types/sdk-types.js';
import { sharedTestDataDir, toFileUrl } from '../../utils/files.js';
import { createLiveConnection, createTempDir } from '../../utils/fixtures.js';
import {
  destroyConnectionAsync,
  executeAsync,
  expectColumnsNames,
  isRunningNewDriverWithBD,
  randomizeName,
} from '../../utils/index.js';

const PUT_COLUMNS = [
  'source',
  'target',
  'sourceSize',
  'targetSize',
  'sourceCompression',
  'targetCompression',
  'status',
  'message',
];
const GET_COLUMNS = ['file', 'size', 'status', 'message'];

// TODO: duplicates put-get.test.ts but doesn't test all compression cases
// maybe they should be merged
describe('PUT GET basic operations', () => {
  let connection: Connection;

  beforeAll(async () => {
    connection = await createLiveConnection({}, false);
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  describe('tests/definitions/shared/put_get/put_get_basic_operations.feature', () => {
    const TEST_FILE = path.join(sharedTestDataDir(), 'compression', 'test_data.csv');

    let stage: string;

    beforeEach(async () => {
      stage = randomizeName('TEST_STAGE_BASIC');
      await executeAsync(connection, `CREATE TEMPORARY STAGE IF NOT EXISTS ${stage}`);
    });

    afterEach(async () => {
      await executeAsync(connection, `DROP STAGE IF EXISTS ${stage}`);
    });

    it('should select data from file uploaded to stage', async () => {
      // Given File is uploaded to stage
      await executeAsync(
        connection,
        `PUT ${toFileUrl(TEST_FILE)} @${stage} AUTO_COMPRESS=TRUE OVERWRITE=TRUE`,
      );

      // When File data is queried using Select command
      const { rows } = await executeAsync(connection, `SELECT $1, $2, $3 FROM @${stage}`);

      // Then File data should be correctly returned
      expect(Object.values(rows[0])).toEqual(['1', '2', '3']);
    });

    it('should list file uploaded to stage', async () => {
      // Given File is uploaded to stage
      await executeAsync(
        connection,
        `PUT ${toFileUrl(TEST_FILE)} @${stage} AUTO_COMPRESS=TRUE OVERWRITE=TRUE`,
      );

      // When Stage content is listed using LS command
      const { rows } = await executeAsync(connection, `LS @${stage}`);

      // Then File should be listed with correct filename
      expect(rows).toHaveLength(1);
      expect(rows[0].name).toContain('test_data.csv.gz');
    });

    it('should get file uploaded to stage', async () => {
      // Given File is uploaded to stage
      await executeAsync(
        connection,
        `PUT ${toFileUrl(TEST_FILE)} @${stage} AUTO_COMPRESS=TRUE OVERWRITE=TRUE`,
      );

      const downloadDir = createTempDir();
      // When File is downloaded using GET command
      const { rows } = await executeAsync(
        connection,
        `GET @${stage}/test_data.csv ${downloadDir.fileUrl()}/`,
      );

      // Then File should be downloaded
      expect(rows[0].status).toBe('DOWNLOADED');
      const downloadedFile = downloadDir.resolve('test_data.csv.gz');
      expect(fs.existsSync(downloadedFile)).toBe(true);

      // And Have correct content
      expect(zlib.gunzipSync(fs.readFileSync(downloadedFile)).toString().trim()).toBe('1,2,3');
    });

    it('should return correct rowset for PUT', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When File is uploaded to stage
      const { rows } = await executeAsync(
        connection,
        `PUT ${toFileUrl(TEST_FILE)} @${stage} AUTO_COMPRESS=TRUE OVERWRITE=TRUE`,
      );

      // Then Rowset for PUT command should be correct
      expect(rows[0]).toMatchObject({
        source: 'test_data.csv',
        target: 'test_data.csv.gz',
        sourceSize: 6,
        targetSize: isRunningNewDriverWithBD('BD#23') ? 32 : 26,
        sourceCompression: isRunningNewDriverWithBD('BD#24') ? 'NONE' : null,
        targetCompression: 'GZIP',
        status: 'UPLOADED',
        message: isRunningNewDriverWithBD('BD#25') ? '' : undefined,
      });
    });

    it('should return correct rowset for GET', async () => {
      // Given File is uploaded to stage
      await executeAsync(
        connection,
        `PUT ${toFileUrl(TEST_FILE)} @${stage} AUTO_COMPRESS=TRUE OVERWRITE=TRUE`,
      );

      const downloadDir = createTempDir();
      // When File is downloaded using GET command
      const { rows } = await executeAsync(
        connection,
        `GET @${stage}/test_data.csv ${downloadDir.fileUrl()}/`,
      );

      // Then Rowset for GET command should be correct
      expect(rows[0]).toMatchObject({
        file: 'test_data.csv.gz',
        size: 26,
        status: 'DOWNLOADED',
        message: isRunningNewDriverWithBD('BD#25') ? '' : undefined,
      });
    });

    it('should return correct column metadata for PUT', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When File is uploaded to stage
      const { statement, rows } = await executeAsync(
        connection,
        `PUT ${toFileUrl(TEST_FILE)} @${stage} AUTO_COMPRESS=TRUE OVERWRITE=TRUE`,
      );

      // Then Column metadata for PUT command should be correct
      expectColumnsNames(statement as RowStatement, PUT_COLUMNS);
      expect(rows[0].status).toBe('UPLOADED');
    });

    it('should return correct column metadata for GET', async () => {
      // Given File is uploaded to stage
      await executeAsync(
        connection,
        `PUT ${toFileUrl(TEST_FILE)} @${stage} AUTO_COMPRESS=TRUE OVERWRITE=TRUE`,
      );

      const downloadDir = createTempDir();
      // When File is downloaded using GET command
      const { statement, rows } = await executeAsync(
        connection,
        `GET @${stage}/test_data.csv ${downloadDir.fileUrl()}/`,
      );

      // Then Column metadata for GET command should be correct
      expectColumnsNames(statement as RowStatement, GET_COLUMNS);
      expect(rows[0].status).toBe('DOWNLOADED');
    });

    it('should upload file to subdirectory in stage', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When File is uploaded to a subdirectory in stage
      const { rows: uploaded } = await executeAsync(
        connection,
        `PUT ${toFileUrl(TEST_FILE)} @${stage}/nested/subdir AUTO_COMPRESS=FALSE`,
      );
      expect(uploaded[0].status).toBe('UPLOADED');

      // Then File should be listed under the subdirectory
      const { rows } = await executeAsync(connection, `LS @${stage}`);
      expect(
        rows.some(
          (row) =>
            String(row.name).includes('nested/subdir') &&
            String(row.name).includes('test_data.csv'),
        ),
      ).toBe(true);
    });

    it('should get file from subdirectory in stage', async () => {
      // Given File is uploaded to a subdirectory in stage
      const { rows: uploaded } = await executeAsync(
        connection,
        `PUT ${toFileUrl(TEST_FILE)} @${stage}/nested/subdir AUTO_COMPRESS=FALSE OVERWRITE=TRUE`,
      );
      expect(uploaded[0].status).toBe('UPLOADED');

      const downloadDir = createTempDir();
      // When All files are downloaded from stage using GET command
      const { rows } = await executeAsync(connection, `GET @${stage}/ ${downloadDir.fileUrl()}/`);

      // Then File should be downloaded flat into the local directory
      expect(rows[0].status).toBe('DOWNLOADED');
      const downloadedFile = downloadDir.resolve('test_data.csv');
      expect(fs.existsSync(downloadedFile)).toBe(true);

      // And Have correct content
      expect(fs.readFileSync(downloadedFile, 'utf8').trim()).toBe('1,2,3');
    });
  });
});
