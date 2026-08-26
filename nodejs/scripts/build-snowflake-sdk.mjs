/* oxlint-disable no-console */
import { readNapiConfig } from '@napi-rs/cli';
import { execFileSync } from 'node:child_process';
import * as fs from 'node:fs/promises';
import * as path from 'node:path';
import { BUILD_DIR, NODE_SDK_PACKAGE, NODE_SDK_PACKAGE_JSON_PATH, ROOT_DIR } from './common.mjs';

const BUILD_SDK_PACKAGE_DIR = path.join(BUILD_DIR, NODE_SDK_PACKAGE.name);
const TSCONFIG_NPM_PATH = path.join(ROOT_DIR, 'tsconfig.npm.json');

// Compiles `src/` into `_build/snowflake-sdk/` as a publish-ready JS package.
// The built package.json is a copy of the workspace manifest with
// `optionalDependencies` added for every `snowflake-sdk-core-<platform>`
// package (napi-rs's install model).
//
// The build runs as the numbered steps marked below.

// 1. Remove the package dir so stale artifacts can't survive across builds.
console.log('Clearing build directory', BUILD_SDK_PACKAGE_DIR);
await fs.rm(BUILD_SDK_PACKAGE_DIR, { recursive: true, force: true });

// 2. Compile `src/` into the package `dist/`. The generated native types must
//    already exist (produced by `build:core`); without them `tsc` cannot type
//    the loader.
execFileSync('npx', ['tsc', '-p', TSCONFIG_NPM_PATH], {
  cwd: ROOT_DIR,
  stdio: 'inherit',
});

// 3. Copy the workspace package.json and add `optionalDependencies`. Platform
//    package names come from `readNapiConfig` (the public API `prePublish`
//    uses). `prePublish` itself is not called: it also validates/publishes
//    every child platform package and needs those dirs to exist.
const { packageName, targets, packageJson } = await readNapiConfig(NODE_SDK_PACKAGE_JSON_PATH);
const publishPackage = {
  ...NODE_SDK_PACKAGE,
  optionalDependencies: {
    ...NODE_SDK_PACKAGE.optionalDependencies,
    ...Object.fromEntries(
      targets.map((target) => [`${packageName}-${target.platformArchABI}`, packageJson.version]),
    ),
  },
};
await fs.writeFile(
  path.join(BUILD_SDK_PACKAGE_DIR, 'package.json'),
  `${JSON.stringify(publishPackage, null, 2)}\n`,
);

// 4. Log the build output
console.log(`build -> _build/${NODE_SDK_PACKAGE.name}/`);
for (const file of await fs.readdir(BUILD_SDK_PACKAGE_DIR)) {
  console.log(`  ${file}`);
}
