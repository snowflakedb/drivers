import fs from 'node:fs';
import path from 'node:path';
import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import type { Connection } from '../types/sdk-types.js';
import {
  createRandomFileName,
  createTestDir,
  deletePathIgnoringErrors,
  toFileUrl,
} from './utils/files.js';
import getTestParameter from './utils/getTestParameter.js';
import {
  createTestConnection,
  destroyConnectionAsync,
  executeAsync,
  getSnowflakeSDK,
  randomizeName,
} from './utils/index.js';

const DATABASE_NAME = getTestParameter('SNOWFLAKE_TEST_DATABASE');
const SCHEMA_NAME = getTestParameter('SNOWFLAKE_TEST_SCHEMA');

const UPLOADED = 'UPLOADED';
const DOWNLOADED = 'DOWNLOADED';

const COL1 = 'C1';
const COL2 = 'C2';
const COL3 = 'C3';
const COL1_DATA = 'FIRST';
const COL2_DATA = 'SECOND';
const COL3_DATA = 'THIRD';
const ROW_DATA = `${COL1_DATA},${COL2_DATA},${COL3_DATA}\n`.repeat(4);
const ROW_DATA_SIZE = 76;

// The file content is always plain CSV; the extension alone tells the driver the file is
// already compressed, so it is uploaded and downloaded untouched.
const compressionCases = [
  { name: 'gzip', extension: '.gz' },
  { name: 'bzip2', extension: '.bz2' },
  { name: 'brotli', extension: '.br' },
  { name: 'deflate', extension: '.deflate' },
  { name: 'raw deflate', extension: '.raw_deflate' },
  { name: 'zstd', extension: '.zst' },
];

async function executeAsyncAndExpectOneRow(
  connection: Connection,
  sqlText: string,
): Promise<Record<string, unknown>> {
  const { rows } = await executeAsync(connection, sqlText);
  expect(rows).toHaveLength(1);
  return rows[0];
}

describe('PUT GET', () => {
  const snowflake = getSnowflakeSDK();
  let connection: Connection;

  beforeAll(async () => {
    connection = createTestConnection(snowflake);
    await connection.connectAsync();
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  describe.for(compressionCases)('$name', ({ name, extension }) => {
    const tableName = randomizeName('TEMP_TABLE');
    const stage = `@${DATABASE_NAME}.${SCHEMA_NAME}.%${tableName}`;

    let testDir: string;
    let uploadedFile: string;
    let downloadDir: string;

    beforeAll(async () => {
      testDir = createTestDir(name);
      const uploadDir = path.join(testDir, 'upload');
      downloadDir = path.join(testDir, 'download');
      fs.mkdirSync(uploadDir);
      fs.mkdirSync(downloadDir);

      uploadedFile = path.join(uploadDir, createRandomFileName({ postfix: extension }));
      fs.writeFileSync(uploadedFile, ROW_DATA);

      await executeAsync(
        connection,
        `CREATE OR REPLACE TEMPORARY TABLE ${tableName} (${COL1} STRING, ${COL2} STRING, ${COL3} STRING)`,
      );
    });

    afterAll(async () => {
      deletePathIgnoringErrors(testDir);
      await executeAsync(connection, `REMOVE ${stage}`);
      // snowflake recommends dropping temporary tables anyway
      await executeAsync(connection, `DROP TABLE IF EXISTS ${tableName}`);
    });

    it('uploads the file to the stage', async () => {
      const uploaded = await executeAsyncAndExpectOneRow(
        connection,
        `PUT ${toFileUrl(uploadedFile)} ${stage}`,
      );
      expect(uploaded.status).toBe(UPLOADED);
    });

    it('copies the staged file into the table', async () => {
      await executeAsync(connection, `COPY INTO ${tableName}`);

      const { rows } = await executeAsync(connection, `SELECT * FROM ${tableName}`);
      expect(rows).toHaveLength(4);
      for (const row of rows) {
        expect(row[COL1]).toBe(COL1_DATA);
        expect(row[COL2]).toBe(COL2_DATA);
        expect(row[COL3]).toBe(COL3_DATA);
      }
    });

    it('downloads the file from the stage', async () => {
      const downloaded = await executeAsyncAndExpectOneRow(
        connection,
        `GET ${stage} ${toFileUrl(downloadDir)}`,
      );
      expect(downloaded.status).toBe(DOWNLOADED);
      expect(downloaded.size).toBe(ROW_DATA_SIZE);

      const downloadedFile = path.join(downloadDir, downloaded.file as string);
      expect(fs.readFileSync(downloadedFile, 'utf8')).toBe(ROW_DATA);
    });
  });
});
