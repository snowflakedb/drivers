/* oxlint-disable no-console */
import { execFileSync } from 'node:child_process';
import * as fs from 'node:fs/promises';
import * as path from 'node:path';
import { BUILD_CORE_PACKAGE_DIR, BUILD_SDK_PACKAGE_DIR, ROOT_DIR } from './common.mjs';

const OLD_SDK_TYPES_PATH = path.join(
  ROOT_DIR,
  'node_modules',
  'snowflake-sdk-old',
  'dist',
  'index.d.ts',
);

const OLD_SDK_MODULE_DECL = "declare module 'snowflake-sdk'";
const OLD_SDK_ALIASED_MODULE_DECL = "declare module 'snowflake-sdk-old'";

async function rewriteOldSdkModuleName() {
  const contents = await fs.readFile(OLD_SDK_TYPES_PATH, 'utf8');
  if (!contents.includes(OLD_SDK_MODULE_DECL)) {
    return;
  }
  await fs.writeFile(
    OLD_SDK_TYPES_PATH,
    contents.replace(OLD_SDK_MODULE_DECL, OLD_SDK_ALIASED_MODULE_DECL),
  );
  console.log(`Renamed module declaration to 'snowflake-sdk-old' in ${OLD_SDK_TYPES_PATH}`);
}

// Links the locally built `snowflake-sdk` and `snowflake-sdk-core` packages into
// the workspace so tests and `tsc` resolve against the freshly built artifacts.
//
// Each `npm link` re-syncs with package.json and removes anything not listed
// there, so both packages must be linked in a single command.

// Only link artifacts that are actually present in `_build`; a package that
// hasn't been built yet is silently skipped rather than treated as an error.
const packageDirs = [];
for (const dir of [BUILD_SDK_PACKAGE_DIR, BUILD_CORE_PACKAGE_DIR]) {
  try {
    await fs.access(dir);
    packageDirs.push(dir);
  } catch {
    // Not built yet; skip it.
  }
}

if (packageDirs.length > 0) {
  execFileSync('npm', ['link', '--silent', '--no-audit', ...packageDirs], {
    cwd: ROOT_DIR,
    stdio: 'inherit',
  });
  console.log('Linked packages:');
  for (const dir of packageDirs) {
    console.log(`  ${dir}`);
  }
  await rewriteOldSdkModuleName();
} else {
  console.log('No built packages found in _build; nothing to link.');
}
