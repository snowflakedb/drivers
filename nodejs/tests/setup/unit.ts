import { execFileSync } from 'node:child_process';
import path from 'node:path';

const ROOT_DIR = path.resolve(import.meta.dirname, '../..');

export default function setup() {
  execFileSync('npm', ['run', 'build:core'], {
    cwd: ROOT_DIR,
    stdio: 'inherit',
  });
  execFileSync('npm', ['link', path.join(ROOT_DIR, '_build', 'snowflake-sdk-core')], {
    cwd: ROOT_DIR,
    stdio: 'inherit',
  });
}
