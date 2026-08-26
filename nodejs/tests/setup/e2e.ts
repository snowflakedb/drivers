import { execFileSync } from 'node:child_process';
import path from 'node:path';

const ROOT_DIR = path.resolve(import.meta.dirname, '../..');

export default function setup() {
  // Build and link for both e2e and e2e-old-driver to resolve SDK types in tests.
  for (const script of ['build:core', 'build:sdk', 'build:link-local']) {
    execFileSync('npm', ['run', script], {
      cwd: ROOT_DIR,
      stdio: 'inherit',
    });
  }
}
