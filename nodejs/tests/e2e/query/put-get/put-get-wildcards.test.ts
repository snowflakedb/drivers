import fs from 'node:fs';
import { describe, it, expect, beforeAll, afterAll, beforeEach, afterEach } from 'vitest';
import type { Connection } from '../../../types/sdk-types.js';
import type { TempDir } from '../../utils/files.js';
import { createLiveConnection, createTempDir } from '../../utils/fixtures.js';
import {
  destroyConnectionAsync,
  executeAsync,
  isRunningNewDriverWithBD,
  randomizeName,
} from '../../utils/index.js';

function writeFiles(dir: TempDir, names: string[]): void {
  for (const name of names) {
    dir.writeFile(name, '1,2,3\n');
  }
}

describe('PUT GET wildcards', () => {
  let connection: Connection;

  beforeAll(async () => {
    connection = await createLiveConnection({}, false);
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  describe('tests/definitions/shared/put_get/put_get_wildcards.feature', () => {
    let stage: string;

    beforeEach(async () => {
      stage = randomizeName('TEST_PUT_GET_WILDCARDS');
      await executeAsync(connection, `CREATE TEMPORARY STAGE IF NOT EXISTS ${stage}`);
    });

    afterEach(async () => {
      await executeAsync(connection, `DROP STAGE IF EXISTS ${stage}`);
    });

    async function stagedBaseNames(): Promise<string[]> {
      const { rows } = await executeAsync(connection, `LS @${stage}`);
      return rows.map((row) => String(row.name).split('/').pop() ?? '');
    }

    it('should upload files that match wildcard question mark pattern', async () => {
      const base = 'test_put_wildcard_question_mark';
      const uploadDir = createTempDir();
      // Given Files matching wildcard pattern
      const matching = [`${base}_1.csv`, `${base}_2.csv`, `${base}_3.csv`];
      writeFiles(uploadDir, matching);

      // And Files not matching wildcard pattern
      const nonMatching = [`${base}_10.csv`, `${base}_abc.csv`];
      writeFiles(uploadDir, nonMatching);

      // When Files are uploaded using command with question mark wildcard
      const execution = executeAsync(
        connection,
        `PUT ${uploadDir.fileUrl(`${base}_?.csv`)} @${stage} AUTO_COMPRESS=FALSE OVERWRITE=TRUE`,
      );

      // The old Node driver expands only `*`, so it stats the literal `?` path and fails (BD#34).
      if (!isRunningNewDriverWithBD('BD#34')) {
        await expect(execution).rejects.toMatchObject({
          error: {
            code: 'ENOENT',
          },
        });
        return;
      }

      const { rows } = await execution;

      // Then Files matching wildcard pattern are uploaded
      expect(rows).toHaveLength(3);
      for (const row of rows) {
        expect(row.status).toBe('UPLOADED');
      }
      const staged = await stagedBaseNames();
      for (const name of matching) {
        expect(staged).toContain(name);
      }

      // And Files not matching wildcard pattern are not uploaded
      for (const name of nonMatching) {
        expect(staged).not.toContain(name);
      }
    });

    it('should upload files that match wildcard star pattern', async () => {
      const base = 'test_put_wildcard_star';
      const uploadDir = createTempDir();
      // Given Files matching wildcard pattern
      const matching = [`${base}_1.csv`, `${base}_2.csv`, `${base}_3.csv`];
      writeFiles(uploadDir, matching);

      // And Files not matching wildcard pattern
      const nonMatching = [`${base}.csv`, `${base}_test.txt`];
      writeFiles(uploadDir, nonMatching);

      // When Files are uploaded using command with star wildcard
      const { rows } = await executeAsync(
        connection,
        `PUT ${uploadDir.fileUrl(`${base}_*.csv`)} @${stage} AUTO_COMPRESS=FALSE OVERWRITE=TRUE`,
      );

      // Then Files matching wildcard pattern are uploaded
      expect(rows).toHaveLength(3);
      for (const row of rows) {
        expect(row.status).toBe('UPLOADED');
      }
      const staged = await stagedBaseNames();
      for (const name of matching) {
        expect(staged).toContain(name);
      }

      // And Files not matching wildcard pattern are not uploaded
      for (const name of nonMatching) {
        expect(staged).not.toContain(name);
      }
    });

    it('should download files that are matching wildcard pattern', async () => {
      const base = 'test_get';
      const uploadDir = createTempDir();
      const downloadDir = createTempDir();
      // Given Files matching wildcard pattern are uploaded
      const matching = [`${base}_1.csv`, `${base}_2.csv`, `${base}_3.csv`];
      writeFiles(uploadDir, matching);

      // And Files not matching wildcard pattern are uploaded
      const nonMatching = [`${base}_10.csv`, `${base}_abc.csv`];
      writeFiles(uploadDir, nonMatching);

      for (const name of [...matching, ...nonMatching]) {
        await executeAsync(
          connection,
          `PUT ${uploadDir.fileUrl(name)} @${stage} AUTO_COMPRESS=TRUE OVERWRITE=TRUE`,
        );
      }

      // When Files are downloaded using command with wildcard
      await executeAsync(
        connection,
        `GET @${stage} ${downloadDir.fileUrl()}/ PATTERN='.*/${base}_.\\.csv\\.gz'`,
      );

      // Then Files matching wildcard pattern are downloaded
      const downloaded = fs.readdirSync(downloadDir.path);
      expect(downloaded).toHaveLength(3);
      for (const name of matching) {
        expect(downloaded).toContain(`${name}.gz`);
      }

      // And Files not matching wildcard pattern are not downloaded
      for (const name of nonMatching) {
        expect(downloaded).not.toContain(`${name}.gz`);
      }
    });
  });
});
