import path from 'node:path';
import { describe, it, expect, beforeAll, afterAll, beforeEach, afterEach } from 'vitest';
import type { Connection } from '../../../types/sdk-types.js';
import { sharedTestDataDir, toFileUrl } from '../../utils/files.js';
import { createLiveConnection } from '../../utils/fixtures.js';
import {
  destroyConnectionAsync,
  executeAsync,
  isRunningNewDriverWithBD,
  randomizeName,
} from '../../utils/index.js';

const COMPRESSION_FILES: Record<string, string> = {
  GZIP: 'test_data.csv.gz',
  BZIP2: 'test_data.csv.bz2',
  BROTLI: 'test_data.csv.br',
  ZSTD: 'test_data.csv.zst',
  DEFLATE: 'test_data.csv.deflate',
  RAW_DEFLATE: 'test_data.csv.raw_deflate',
};

describe('PUT GET source compression', () => {
  let connection: Connection;

  function compressionFile(name: string): string {
    return path.join(sharedTestDataDir(), 'compression', name);
  }

  beforeAll(async () => {
    connection = await createLiveConnection({}, false);
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  describe('tests/definitions/shared/put_get/put_get_source_compression.feature', () => {
    let stage: string;

    beforeEach(async () => {
      stage = randomizeName('TEST_STAGE_SRC_COMPRESSION');
      await executeAsync(connection, `CREATE TEMPORARY STAGE IF NOT EXISTS ${stage}`);
    });

    afterEach(async () => {
      await executeAsync(connection, `DROP STAGE IF EXISTS ${stage}`);
    });

    function uploadedRow(expected: {
      source: string;
      sourceCompression: string;
      target: string;
      targetCompression: string;
    }): Record<string, unknown> {
      return {
        source: expected.source,
        target: expected.target,
        sourceCompression: isRunningNewDriverWithBD('BD#24') ? expected.sourceCompression : null,
        targetCompression: expected.targetCompression,
        status: 'UPLOADED',
      };
    }

    it('should auto-detect standard compression types when SOURCE_COMPRESSION set to AUTO_DETECT', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And File with standard type (GZIP, BZIP2, BROTLI, ZSTD, DEFLATE)
      const cases = ['GZIP', 'BZIP2', 'BROTLI', 'ZSTD', 'DEFLATE'];

      for (const type of cases) {
        const filename = COMPRESSION_FILES[type];

        // When File is uploaded with SOURCE_COMPRESSION set to AUTO_DETECT
        const execution = executeAsync(
          connection,
          `PUT ${toFileUrl(compressionFile(filename))} @${stage} SOURCE_COMPRESSION=AUTO_DETECT`,
        );

        // Then Target compression has correct type and all PUT results are correct
        if (isRunningNewDriverWithBD('BD#33')) {
          await expect(execution).resolves.toMatchObject({
            rows: [
              uploadedRow({
                source: filename,
                sourceCompression: type,
                target: filename,
                targetCompression: type,
              }),
            ],
          });
        } else {
          await expect(execution).rejects.toMatchObject({
            error: {
              name: 'TypeError',
              message: "Cannot read properties of undefined (reading 'is_supported')",
            },
          });
        }
      }
    }, 120_000);

    it('should upload compressed files with SOURCE_COMPRESSION set to explicit types', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And File with standard type (GZIP, BZIP2, BROTLI, ZSTD, DEFLATE, RAW_DEFLATE)
      const cases = ['GZIP', 'BZIP2', 'BROTLI', 'ZSTD', 'DEFLATE', 'RAW_DEFLATE'];

      for (const type of cases) {
        const filename = COMPRESSION_FILES[type];

        // When File is uploaded with SOURCE_COMPRESSION set to explicit type
        const execution = executeAsync(
          connection,
          `PUT ${toFileUrl(compressionFile(filename))} @${stage} SOURCE_COMPRESSION=${type}`,
        );

        // Then Target compression has correct type and all PUT results are correct
        if (type === 'BROTLI' && !isRunningNewDriverWithBD('BD#32')) {
          await expect(execution).rejects.toMatchObject({
            error: {
              name: 'TypeError',
              message: "Cannot read properties of undefined (reading 'is_supported')",
            },
          });
        } else {
          await expect(execution).resolves.toMatchObject({
            rows: [
              uploadedRow({
                source: filename,
                sourceCompression: type,
                target: filename,
                targetCompression: type,
              }),
            ],
          });
        }
      }
    }, 120_000);

    it('should not compress file when SOURCE_COMPRESSION set to AUTO_DETECT and AUTO_COMPRESS set to FALSE', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And Uncompressed file
      const uncompressedFile = compressionFile('test_data.csv');

      // When File is uploaded with SOURCE_COMPRESSION set to AUTO_DETECT and AUTO_COMPRESS set to FALSE
      const execution = executeAsync(
        connection,
        `PUT ${toFileUrl(uncompressedFile)} @${stage} SOURCE_COMPRESSION=AUTO_DETECT AUTO_COMPRESS=FALSE`,
      );

      // Then File is not compressed
      if (isRunningNewDriverWithBD('BD#33')) {
        await expect(execution).resolves.toMatchObject({
          rows: [
            uploadedRow({
              source: 'test_data.csv',
              sourceCompression: 'NONE',
              target: 'test_data.csv',
              targetCompression: 'NONE',
            }),
          ],
        });
      } else {
        await expect(execution).rejects.toMatchObject({
          error: {
            name: 'TypeError',
            message: "Cannot read properties of undefined (reading 'is_supported')",
          },
        });
      }
    });

    it('should not compress file when SOURCE_COMPRESSION set to NONE and AUTO_COMPRESS set to FALSE', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And Uncompressed file
      const uncompressedFile = compressionFile('test_data.csv');

      // When File is uploaded with SOURCE_COMPRESSION set to NONE and AUTO_COMPRESS set to FALSE
      const execution = executeAsync(
        connection,
        `PUT ${toFileUrl(uncompressedFile)} @${stage} SOURCE_COMPRESSION=NONE AUTO_COMPRESS=FALSE`,
      );

      // Then File is not compressed
      if (isRunningNewDriverWithBD('BD#33')) {
        await expect(execution).resolves.toMatchObject({
          rows: [
            uploadedRow({
              source: 'test_data.csv',
              sourceCompression: 'NONE',
              target: 'test_data.csv',
              targetCompression: 'NONE',
            }),
          ],
        });
      } else {
        await expect(execution).rejects.toMatchObject({
          error: {
            name: 'TypeError',
            message: "Cannot read properties of undefined (reading 'is_supported')",
          },
        });
      }
    });

    it('should compress uncompressed file when SOURCE_COMPRESSION set to AUTO_DETECT and AUTO_COMPRESS set to TRUE', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And Uncompressed file
      const uncompressedFile = compressionFile('test_data.csv');

      // When File is uploaded with SOURCE_COMPRESSION set to AUTO_DETECT and AUTO_COMPRESS set to TRUE
      const execution = executeAsync(
        connection,
        `PUT ${toFileUrl(uncompressedFile)} @${stage} SOURCE_COMPRESSION=AUTO_DETECT AUTO_COMPRESS=TRUE`,
      );

      // Then Target compression has GZIP type and all PUT results are correct
      if (isRunningNewDriverWithBD('BD#33')) {
        await expect(execution).resolves.toMatchObject({
          rows: [
            uploadedRow({
              source: 'test_data.csv',
              sourceCompression: 'NONE',
              target: 'test_data.csv.gz',
              targetCompression: 'GZIP',
            }),
          ],
        });
      } else {
        await expect(execution).rejects.toMatchObject({
          error: {
            name: 'TypeError',
            message: "Cannot read properties of undefined (reading 'is_supported')",
          },
        });
      }
    });

    it('should compress uncompressed file when SOURCE_COMPRESSION set to NONE and AUTO_COMPRESS set to TRUE', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And Uncompressed file
      const uncompressedFile = compressionFile('test_data.csv');

      // When File is uploaded with SOURCE_COMPRESSION set to NONE and AUTO_COMPRESS set to TRUE
      const execution = executeAsync(
        connection,
        `PUT ${toFileUrl(uncompressedFile)} @${stage} SOURCE_COMPRESSION=NONE AUTO_COMPRESS=TRUE`,
      );

      // Then Target compression has GZIP type and all PUT results are correct
      if (isRunningNewDriverWithBD('BD#33')) {
        await expect(execution).resolves.toMatchObject({
          rows: [
            uploadedRow({
              source: 'test_data.csv',
              sourceCompression: 'NONE',
              target: 'test_data.csv.gz',
              targetCompression: 'GZIP',
            }),
          ],
        });
      } else {
        await expect(execution).rejects.toMatchObject({
          error: {
            name: 'TypeError',
            message: "Cannot read properties of undefined (reading 'is_supported')",
          },
        });
      }
    });

    // TODO: Return to this test after implementing proper error handling first.
    // We need to decide on the error code that should be returned in new driver.
    it.todo('should return error for unsupported compression type', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And File compressed with unsupported format
      const unsupportedFile = compressionFile('test_data.csv.xz');

      // When File is uploaded with SOURCE_COMPRESSION set to AUTO_DETECT
      const execution = executeAsync(
        connection,
        `PUT ${toFileUrl(unsupportedFile)} @${stage} SOURCE_COMPRESSION=AUTO_DETECT`,
      );

      // Then Unsupported compression error is thrown
      if (isRunningNewDriverWithBD('BD#33')) {
        await expect(execution).rejects.toMatchObject({
          error: {
            message: 'Failed to process query response: Failed to upload files',
            cause: {
              message: 'Unsupported compression type: XZ',
            },
          },
        });
      } else {
        await expect(execution).rejects.toMatchObject({
          error: {
            name: 'TypeError',
            message: "Cannot read properties of undefined (reading 'is_supported')",
          },
        });
      }
    });
  });

  // TODO: remove when running tests for old driver is removed
  describe('SOURCE_COMPRESSION mime-subtype alias', () => {
    it('should treat SOURCE_COMPRESSION=BR as BROTLI on the old driver but reject it on the new driver', async () => {
      const stage = randomizeName('TEST_STAGE_SRC_COMPRESSION_BR');
      await executeAsync(connection, `CREATE TEMPORARY STAGE IF NOT EXISTS ${stage}`);
      try {
        const execution = executeAsync(
          connection,
          `PUT ${toFileUrl(compressionFile('test_data.csv.br'))} @${stage} SOURCE_COMPRESSION=BR`,
        );

        if (isRunningNewDriverWithBD('BD#32')) {
          await expect(execution).rejects.toMatchObject({
            error: {
              message: 'Failed to process query response: Failed to prepare file transfer data',
              cause: {
                message: 'Invalid Snowflake response: Unknown source compression type: BR',
              },
            },
          });
        } else {
          await expect(execution).resolves.toMatchObject({
            rows: [
              {
                source: 'test_data.csv.br',
                target: 'test_data.csv.br',
                targetCompression: 'BROTLI',
                status: 'UPLOADED',
              },
            ],
          });
        }
      } finally {
        await executeAsync(connection, `DROP STAGE IF EXISTS ${stage}`);
      }
    });
  });
});
