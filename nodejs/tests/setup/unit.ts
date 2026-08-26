import { execFileSync } from 'node:child_process';
import path from 'node:path';

const ROOT_DIR = path.resolve(import.meta.dirname, '../..');

export default function setup() {
  for (const script of ['build:core', 'build:link-local']) {
    execFileSync('npm', ['run', script], {
      cwd: ROOT_DIR,
      stdio: 'inherit',
    });
  }
}
