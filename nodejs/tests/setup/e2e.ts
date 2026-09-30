import { execFileSync } from 'node:child_process';
import path from 'node:path';

const ROOT_DIR = path.resolve(import.meta.dirname, '../..');

export default function setup() {
  // WIF unpacks a prebuilt addon; local runs may already have `_build`.
  if (process.env.SKIP_NODEJS_BUILD === 'true') {
    return;
  }

  // Build and link for both e2e and e2e-old-driver to resolve SDK types in tests.
  for (const script of ['build:core', 'build:sdk', 'build:link-local']) {
    execFileSync('npm', ['run', script], {
      cwd: ROOT_DIR,
      stdio: 'inherit',
    });
  }
}
