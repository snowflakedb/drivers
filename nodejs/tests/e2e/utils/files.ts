import { randomUUID } from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

export function createRandomFileName(options: { prefix?: string; postfix?: string } = {}): string {
  return `${options.prefix ?? ''}${randomUUID()}${options.postfix ?? ''}`;
}

export function createTestDir(testName: string): string {
  const slug = testName
    // non-alphanumerics → one dash
    .replace(/[^a-z0-9]+/gi, '-')
    // drop dashes at the start and end
    .replace(/^-+|-+$/g, '')
    .toLowerCase();
  return fs.mkdtempSync(path.join(os.tmpdir(), `${slug}-`));
}

export function deletePathIgnoringErrors(target: string): void {
  try {
    fs.rmSync(target, { force: true, recursive: true });
  } catch {
    // leftover temp files are not worth failing a test over
  }
}

/**
 * Builds the `file://` argument for PUT and GET.
 *
 * On Windows `os.tmpdir()` resolves to a short path containing `~`, which the drivers reject, so
 * the temp directory prefix is replaced with the expanded user profile one. Only paths inside the
 * temp directory are handled.
 */
export function toFileUrl(localPath: string): string {
  if (process.platform !== 'win32') {
    return `file://${localPath}`;
  }
  const insideTempDir = path.relative(os.tmpdir(), localPath);
  const expandedTempDir = path.join(process.env.USERPROFILE ?? '', 'AppData', 'Local', 'Temp');
  return `file://${path.join(expandedTempDir, insideTempDir)}`;
}
