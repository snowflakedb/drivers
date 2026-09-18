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

describe('tests/definitions/shared/put_get/put_get_errors.feature', () => {
  let connection: Connection;
  let stage: string;

  beforeAll(async () => {
    connection = await createLiveConnection({}, false);
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  beforeEach(async () => {
    stage = randomizeName('TEST_PUT_GET_ERRORS');
    await executeAsync(connection, `CREATE TEMPORARY STAGE IF NOT EXISTS ${stage}`);
  });

  afterEach(async () => {
    await executeAsync(connection, `DROP STAGE IF EXISTS ${stage}`);
  });

  // TODO: The new driver reports a generic top-level message ("Failed to process query response:
  // Failed to upload files") and carries the missing-file reason on error.cause ("File does not
  // exist"), whereas the old Node, Python, and JDBC drivers surface that reason on the top-level
  // error. Re-enable once the new driver properly reports errors.
  it.todo('should return error when putting nonexistent local file', async () => {
    // Given A stage is created
    void stage;

    // When PUT is executed with a path to a nonexistent local file
    const uploadDir = createTempDir();
    const nonexistent = uploadDir.resolve('nonexistent.csv');
    const execution = executeAsync(connection, `PUT ${toFileUrl(nonexistent)} @${stage}`);

    // Then An error is raised indicating the local file does not exist
    await expect(execution).rejects.toMatchObject({
      error: {
        message: expect.stringMatching(/does not exist|no such file/i),
      },
    });
  });

  it('should return error when getting nonexistent file from stage', async () => {
    // Given An empty stage is created
    void stage;

    // When GET is executed for a file that does not exist in stage
    const downloadDir = createTempDir();
    const execution = executeAsync(
      connection,
      `GET @${stage}/nonexistent.csv ${downloadDir.fileUrl()}/`,
    );

    // Then An error is raised indicating the remote file does not exist
    if (isRunningNewDriverWithBD('BD#35')) {
      await expect(execution).rejects.toMatchObject({
        error: {
          message:
            'Failed to process query response: While getting file(s) there was an error: the file does not exist',
          cause: {
            message: 'While getting file(s) there was an error: the file does not exist',
            code: undefined,
          },
          code: undefined,
        },
      });
    } else {
      await expect(execution).resolves.toMatchObject({
        rows: [],
      });
    }
  });
});
