import { execFileSync } from 'node:child_process';
import path from 'node:path';

const ROOT_DIR = path.resolve(import.meta.dirname, '../..');

export default function setup() {
  execFileSync('npm', ['run', 'build:sdk'], {
    cwd: ROOT_DIR,
    stdio: 'inherit',
  });
  execFileSync('npm', ['run', 'build:core'], {
    cwd: ROOT_DIR,
    stdio: 'inherit',
  });
  // Each npm link re-syncs with package.json and removes anything not listed there, so
  // snowflake-sdk and snowflake-sdk-core must be linked in a single command.
  execFileSync(
    'npm',
    [
      'link',
      path.join(ROOT_DIR, '_build', 'snowflake-sdk'),
      path.join(ROOT_DIR, '_build', 'snowflake-sdk-core'),
    ],
    {
      cwd: ROOT_DIR,
      stdio: 'inherit',
    },
  );
}
