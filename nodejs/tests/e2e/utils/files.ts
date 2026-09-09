import { randomUUID } from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

/**
 * Resolves the shared directory that the cross-driver Gherkin features reference
 */
export function sharedTestDataDir(): string {
  let dir = path.dirname(fileURLToPath(import.meta.url));
  while (dir !== path.dirname(dir)) {
    const candidate = path.join(dir, 'tests', 'test_data', 'generated_test_data');
    if (fs.existsSync(candidate)) {
      return candidate;
    }
    dir = path.dirname(dir);
  }
  throw new Error('Could not locate tests/test_data/generated_test_data');
}

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

export interface TempDirOptions {
  root?: string;
  prefix?: string;
}

export class TempDir {
  readonly path: string;

  constructor(options: TempDirOptions = {}) {
    const prefix = options.prefix ?? 'tmp';
    const separatedPrefix = prefix.endsWith('-') ? prefix : `${prefix}-`;
    this.path = fs.mkdtempSync(path.join(options.root ?? os.tmpdir(), separatedPrefix));
  }

  get name(): string {
    return path.basename(this.path);
  }

  #toPosix(p: string): string {
    return p.split(path.sep).join('/');
  }

  resolve(relativePath: string): string {
    return this.#toPosix(path.join(this.path, relativePath));
  }

  mkdir(relativePath: string): string {
    const dir = this.resolve(relativePath);
    fs.mkdirSync(dir, { recursive: true });
    return dir;
  }

  writeFile(relativeFilePath: string, content = 'a,b,c\n'): string {
    const filePath = this.resolve(relativeFilePath);
    fs.mkdirSync(path.dirname(filePath), { recursive: true });
    fs.writeFileSync(filePath, content);
    return filePath;
  }

  linkFile(targetRelativePath: string, linkRelativePath: string): string {
    const link = this.resolve(linkRelativePath);
    fs.symlinkSync(this.resolve(targetRelativePath), link);
    return link;
  }

  fileUrl(relativePath: string): string {
    return this.#toPosix(toFileUrl(this.resolve(relativePath)));
  }

  relative(base: string, relativePath: string): string {
    return this.#toPosix(path.relative(base, this.resolve(relativePath)));
  }

  cleanup(): void {
    deletePathIgnoringErrors(this.path);
  }
}
