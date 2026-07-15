import { NapiCli } from '@napi-rs/cli';
/* oxlint-disable no-console */
import * as fs from 'node:fs/promises';
import * as path from 'node:path';
import * as url from 'node:url';

// Compiles the `nodejs_bridge` Rust crate into the native core of the
// `snowflake-sdk` npm package, producing two artifacts in
// `_build/<napi.packageName>/` (i.e. `_build/snowflake-sdk-core/`):
//   - a platform-specific `.node` binary (release build for the host target), and
//   - a `.d.ts` file with the TypeScript types for its N-API exports.
//
// How it works:
//   1. Read the `napi` config (packageName/binaryName/targets) from the SDK's
//      package.json, so this script and the published package stay in sync.
//   2. Clean the output dir, then invoke `@napi-rs/cli` build against the crate's
//      Cargo.toml. `platform: true` names the `.node` file per host target;
//      `noJsBinding: true` skips the generated JS loader shim (the SDK ships its
//      own loader, so the `.d.ts` is the only JS-facing artifact).
const ROOT_DIR = path.dirname(path.dirname(url.fileURLToPath(import.meta.url)));
const NODE_SDK_PACKAGE_JSON_PATH = path.join(ROOT_DIR, 'package.json');
const { napi: NAPI_CONFIG } = JSON.parse(await fs.readFile(NODE_SDK_PACKAGE_JSON_PATH, 'utf8'));
const BUILD_CORE_BINARY_DIR = path.join(ROOT_DIR, '_build', NAPI_CONFIG.packageName);

// Start from a clean output dir so stale artifacts can't survive across builds.
await fs.rm(BUILD_CORE_BINARY_DIR, { recursive: true, force: true });

const cli = new NapiCli();
const { task } = await cli.build({
  platform: true,
  release: true,
  cargoName: 'nodejs_bridge',
  manifestPath: path.join(ROOT_DIR, '..', 'nodejs_bridge', 'Cargo.toml'),
  packageJsonPath: NODE_SDK_PACKAGE_JSON_PATH,
  outputDir: BUILD_CORE_BINARY_DIR,
  cwd: ROOT_DIR,
  noJsBinding: true,
});
await task;

console.log(`build -> _build/${NAPI_CONFIG.packageName}/`);
for (const file of await fs.readdir(BUILD_CORE_BINARY_DIR)) {
  console.log(`  ${file}`);
}
