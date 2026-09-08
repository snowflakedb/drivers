import fs from 'node:fs';
import path from 'node:path';
import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import type { Connection, RowStatement } from '../../../types/sdk-types.js';
import {
  createRandomFileName,
  createTestDir,
  deletePathIgnoringErrors,
  toFileUrl,
} from '../../utils/files.js';
import getTestParameter from '../../utils/getTestParameter.js';
import {
  createTestConnection,
  destroyConnectionAsync,
  executeAsync,
  expectColumnsNames,
  isRunningNewDriverWithBD,
  randomizeName,
} from '../../utils/index.js';

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
const ENCRYPTED_ROW_DATA_SIZE = 80;

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

// The file content is always plain CSV; the extension alone tells the driver the file is
// already compressed, so it is uploaded and downloaded untouched.
const compressionCases = [
  { name: 'gzip', extension: '.gz', targetCompression: 'GZIP' },
  { name: 'bzip2', extension: '.bz2', targetCompression: 'BZIP2' },
  { name: 'brotli', extension: '.br', targetCompression: 'BROTLI' },
  { name: 'deflate', extension: '.deflate', targetCompression: 'DEFLATE' },
  { name: 'raw deflate', extension: '.raw_deflate', targetCompression: 'RAW_DEFLATE' },
  { name: 'zstd', extension: '.zst', targetCompression: 'ZSTD' },
];

describe('PUT GET', () => {
  let connection: Connection;

  beforeAll(async () => {
    connection = createTestConnection();
    await connection.connectAsync();
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  describe.for(compressionCases)('$name', ({ name, extension, targetCompression }) => {
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
      const { statement, rows } = await executeAsync(
        connection,
        `PUT ${toFileUrl(uploadedFile)} ${stage}`,
      );
      expectColumnsNames(statement as RowStatement, PUT_COLUMNS);

      expect(rows).toMatchObject([
        {
          source: path.basename(uploadedFile),
          target: path.basename(uploadedFile),
          sourceSize: ROW_DATA_SIZE,
          targetSize: isRunningNewDriverWithBD('BD#23') ? ENCRYPTED_ROW_DATA_SIZE : ROW_DATA_SIZE,
          sourceCompression: isRunningNewDriverWithBD('BD#24') ? targetCompression : null,
          targetCompression,
          status: UPLOADED,
          message: isRunningNewDriverWithBD('BD#25') ? '' : undefined,
        },
      ]);
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
      const { statement, rows } = await executeAsync(
        connection,
        `GET ${stage} ${toFileUrl(downloadDir)}`,
      );
      expectColumnsNames(statement as RowStatement, GET_COLUMNS);

      expect(rows).toMatchObject([
        {
          file: path.basename(uploadedFile),
          size: ROW_DATA_SIZE,
          status: DOWNLOADED,
          message: isRunningNewDriverWithBD('BD#25') ? '' : undefined,
        },
      ]);

      const downloadedFile = path.join(downloadDir, rows[0].file as string);
      expect(fs.readFileSync(downloadedFile, 'utf8')).toBe(ROW_DATA);
    });
  });
});
