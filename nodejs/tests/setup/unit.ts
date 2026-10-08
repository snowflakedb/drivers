import { execFileSync } from 'node:child_process';
import path from 'node:path';

const ROOT_DIR = path.resolve(import.meta.dirname, '../..');

export default function setup() {
  const npmCli = process.env.npm_execpath;
  for (const script of ['build:core', 'build:link-local']) {
    execFileSync(npmCli ? process.execPath : 'npm', [...(npmCli ? [npmCli] : []), 'run', script], {
      cwd: ROOT_DIR,
      stdio: 'inherit',
      shell: process.platform === 'win32' && !npmCli,
    });
  }
}
