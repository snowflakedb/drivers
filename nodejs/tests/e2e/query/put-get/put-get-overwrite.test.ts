import path from 'node:path';
import { describe, it, expect, beforeAll, afterAll, beforeEach, afterEach } from 'vitest';
import type { Connection } from '../../../types/sdk-types.js';
import { sharedTestDataDir, toFileUrl } from '../../utils/files.js';
import {
  createTestConnection,
  destroyConnectionAsync,
  executeAsync,
  randomizeName,
} from '../../utils/index.js';

describe('PUT GET overwrite', () => {
  let connection: Connection;

  beforeAll(async () => {
    connection = createTestConnection();
    await connection.connectAsync();
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  describe('tests/definitions/shared/put_get/put_get_overwrite.feature', () => {
    const OVERWRITE_ORIGINAL_FILE = path.join(
      sharedTestDataDir(),
      'overwrite',
      'original',
      'test_data.csv',
    );
    const OVERWRITE_UPDATED_FILE = path.join(
      sharedTestDataDir(),
      'overwrite',
      'updated',
      'test_data.csv',
    );

    let stage: string;

    beforeEach(async () => {
      stage = randomizeName('TEST_PUT_GET_OVERWRITE');
      await executeAsync(connection, `CREATE TEMPORARY STAGE IF NOT EXISTS ${stage}`);
    });

    afterEach(async () => {
      await executeAsync(connection, `DROP STAGE IF EXISTS ${stage}`);
    });

    it('should overwrite file when OVERWRITE is set to true', async () => {
      // Given File is uploaded to stage
      const { rows: initial } = await executeAsync(
        connection,
        `PUT ${toFileUrl(OVERWRITE_ORIGINAL_FILE)} @${stage}`,
      );
      expect(initial[0].status).toBe('UPLOADED');

      // When Updated file is uploaded with OVERWRITE set to true
      const { rows: updated } = await executeAsync(
        connection,
        `PUT ${toFileUrl(OVERWRITE_UPDATED_FILE)} @${stage} OVERWRITE=TRUE`,
      );

      // Then UPLOADED status is returned
      expect(updated[0].status).toBe('UPLOADED');

      // And File was overwritten
      const { rows: staged } = await executeAsync(connection, `SELECT $1, $2, $3 FROM @${stage}`);
      expect(Object.values(staged[0])).toEqual(['updated', 'test', 'data']);
    });

    it('should not overwrite file when OVERWRITE is set to false', async () => {
      // Given File is uploaded to stage
      const { rows: initial } = await executeAsync(
        connection,
        `PUT ${toFileUrl(OVERWRITE_ORIGINAL_FILE)} @${stage}`,
      );
      expect(initial[0].status).toBe('UPLOADED');

      // When Updated file is uploaded with OVERWRITE set to false
      const { rows: updated } = await executeAsync(
        connection,
        `PUT ${toFileUrl(OVERWRITE_UPDATED_FILE)} @${stage} OVERWRITE=FALSE`,
      );

      // Then SKIPPED status is returned
      expect(updated[0].status).toBe('SKIPPED');

      // And File was not overwritten
      const { rows: staged } = await executeAsync(connection, `SELECT $1, $2, $3 FROM @${stage}`);
      expect(Object.values(staged[0])).toEqual(['original', 'test', 'data']);
    });
  });
});
