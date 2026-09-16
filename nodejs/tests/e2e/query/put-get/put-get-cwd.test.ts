import fs from 'node:fs';
import { describe, it, expect, beforeAll, afterAll, beforeEach, afterEach } from 'vitest';
import type { Connection } from '../../../types/sdk-types.js';
import { createLiveConnection, createTempDir } from '../../utils/fixtures.js';
import { destroyConnectionAsync, executeAsync, randomizeName } from '../../utils/index.js';

describe('tests/definitions/shared/put_get/put_get_cwd.feature', () => {
  let connection: Connection;
  let stage: string;

  beforeAll(async () => {
    connection = await createLiveConnection({}, false);
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  beforeEach(async () => {
    stage = randomizeName('TEST_PUT_CWD');
    await executeAsync(connection, `CREATE TEMPORARY STAGE IF NOT EXISTS ${stage}`);
  });

  afterEach(async () => {
    await executeAsync(connection, `DROP STAGE IF EXISTS ${stage}`);
  });

  it('should ignore cwd when the PUT source path is absolute', async () => {
    // Given A source file exists at an absolute path
    const fileDir = createTempDir();
    fileDir.writeFile('abs.csv');
    // And cwd points at a different empty directory
    const otherDir = createTempDir();

    // When PUT is executed with that absolute file URI and cwd
    const { rows } = await executeAsync(
      connection,
      `PUT ${fileDir.fileUrl('abs.csv')} @${stage} AUTO_COMPRESS=FALSE`,
      { cwd: otherDir.path },
    );

    // Then The file is uploaded from the absolute path
    expect(rows[0].status).toBe('UPLOADED');
    expect(rows[0].target).toBe('abs.csv');
  });

  it('should resolve a relative PUT source path against a relative cwd', async () => {
    // Given A source file exists under a directory relative to the process working directory
    const workDir = createTempDir({ root: process.cwd() });
    workDir.writeFile('data.csv');

    // When PUT is executed with a relative file URI and that relative cwd
    const { rows } = await executeAsync(
      connection,
      `PUT file://./data.csv @${stage} AUTO_COMPRESS=FALSE`,
      { cwd: workDir.relative(process.cwd(), '') },
    );

    // Then The file is uploaded from the joined path
    expect(rows[0].status).toBe('UPLOADED');
    expect(rows[0].target).toBe('data.csv');
  });

  it('should resolve a relative PUT source path against an absolute cwd', async () => {
    // Given A source file exists in a temporary directory
    const workDir = createTempDir();
    workDir.writeFile('data.csv');

    // When PUT is executed with a relative file URI and that directory as cwd
    const { rows } = await executeAsync(
      connection,
      `PUT file://./data.csv @${stage} AUTO_COMPRESS=FALSE`,
      { cwd: workDir.path },
    );

    // Then The file is uploaded from the joined path
    expect(rows[0].status).toBe('UPLOADED');
    expect(rows[0].target).toBe('data.csv');
  });

  it('should ignore cwd when the GET destination path is absolute', async () => {
    // Given A file exists on a stage
    const uploadDir = createTempDir();
    uploadDir.writeFile('data.csv');
    await executeAsync(
      connection,
      `PUT ${uploadDir.fileUrl('data.csv')} @${stage} AUTO_COMPRESS=FALSE`,
    );
    // And cwd points at a different empty directory
    const otherDir = createTempDir();

    // When GET is executed with an absolute destination URI and cwd
    const destDir = createTempDir();
    const { rows } = await executeAsync(
      connection,
      `GET @${stage}/data.csv ${destDir.fileUrl()}/`,
      { cwd: otherDir.path },
    );

    // Then The file is downloaded to the absolute path
    expect(rows[0].status).toBe('DOWNLOADED');
    expect(rows[0].file).toBe('data.csv');
    expect(fs.existsSync(destDir.resolve('data.csv'))).toBe(true);
    expect(fs.existsSync(otherDir.resolve('data.csv'))).toBe(false);
  });

  it('should resolve a relative GET destination against a relative cwd', async () => {
    // Given A file exists on a stage
    const uploadDir = createTempDir();
    uploadDir.writeFile('data.csv');
    await executeAsync(
      connection,
      `PUT ${uploadDir.fileUrl('data.csv')} @${stage} AUTO_COMPRESS=FALSE`,
    );
    // And a destination directory exists relative to the process working directory
    const destDir = createTempDir({ root: process.cwd() });

    // When GET is executed with a relative destination URI and that relative cwd
    const { rows } = await executeAsync(connection, `GET @${stage}/data.csv file://./`, {
      cwd: destDir.relative(process.cwd(), ''),
    });

    // Then The file is downloaded to the joined path
    expect(rows[0].status).toBe('DOWNLOADED');
    expect(rows[0].file).toBe('data.csv');
    expect(fs.existsSync(destDir.resolve('data.csv'))).toBe(true);
  });

  it('should resolve a relative GET destination against an absolute cwd', async () => {
    // Given A file exists on a stage
    const uploadDir = createTempDir();
    uploadDir.writeFile('data.csv');
    await executeAsync(
      connection,
      `PUT ${uploadDir.fileUrl('data.csv')} @${stage} AUTO_COMPRESS=FALSE`,
    );
    // And a destination directory exists in a temporary directory
    const destDir = createTempDir();

    // When GET is executed with a relative destination URI and that directory as cwd
    const { rows } = await executeAsync(connection, `GET @${stage}/data.csv file://./`, {
      cwd: destDir.path,
    });

    // Then The file is downloaded to the joined path
    expect(rows[0].status).toBe('DOWNLOADED');
    expect(rows[0].file).toBe('data.csv');
    expect(fs.existsSync(destDir.resolve('data.csv'))).toBe(true);
  });
});
