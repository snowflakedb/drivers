/* oxlint-disable no-console */
import { NapiCli } from '@napi-rs/cli';
import * as fs from 'node:fs/promises';
import * as path from 'node:path';
import * as url from 'node:url';

// Compiles the `nodejs_bridge` Rust crate into a self-contained platform
// package in `_build/<napi.packageName>/` (i.e. `_build/snowflake-sdk-core/`):
//   - a platform-specific `.node` binary (release build for the host target),
//   - a `.d.ts` file (emitted by `@napi-rs/cli`) with the TypeScript types for
//     the crate's N-API exports, and
//   - a `package.json` named `snowflake-sdk-core-<triple>` with os/cpu guards,
//     `main` and `files` pointing at the `.node`.
//
// The build runs as the numbered steps marked below.

// 1. Read the `napi` config (packageName/binaryName/targets) from the SDK's
//    package.json and derive the output paths.
const __dirname = path.dirname(url.fileURLToPath(import.meta.url));
const ROOT_DIR = path.resolve(__dirname, '..');
const NODE_SDK_PACKAGE_JSON_PATH = path.join(ROOT_DIR, 'package.json');
const { napi: NAPI_CONFIG } = JSON.parse(await fs.readFile(NODE_SDK_PACKAGE_JSON_PATH, 'utf8'));
const BUILD_DIR = path.join(ROOT_DIR, '_build');
const BUILD_CORE_BINARY_DIR = path.join(BUILD_DIR, NAPI_CONFIG.packageName);
const NAPI_PLACEHOLDER_PACKAGES_DIR = path.join(BUILD_DIR, 'napi-placeholder-packages');

// 2. Clean the target dir in place (delete its contents, not the dir itself) so
//    stale artifacts can't survive across builds while an existing `npm link`
//    symlink pointing at this dir keeps resolving.
await fs.mkdir(BUILD_CORE_BINARY_DIR, { recursive: true });
for (const entry of await fs.readdir(BUILD_CORE_BINARY_DIR)) {
  await fs.rm(path.join(BUILD_CORE_BINARY_DIR, entry), {
    recursive: true,
    force: true,
  });
}

// 3. Build the crate into the target dir:
//    - `platform: true` names the `.node` file per host target.
//    - `noJsBinding: true` skips the generated JS loader shim, so the `.d.ts` is
//      the only JS-facing artifact.
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

// 4. Copy the host `package.json` into the target dir:
//    - Read the platform triple back from the produced `.node` filename.
//    - Run `napi create-npm-dirs`, which writes one subdir per configured target
//      under `NAPI_PLACEHOLDER_PACKAGES_DIR`, each holding a placeholder
//      package.json for that platform's N-API package (with os/cpu guards).
//    - Copy only the host target's manifest into the package dir.
const binaryFileName = (await fs.readdir(BUILD_CORE_BINARY_DIR)).find((file) =>
  file.endsWith('.node'),
);
const platformTriple = binaryFileName.slice(`${NAPI_CONFIG.binaryName}.`.length, -'.node'.length);
// TODO: We might want to optimize this in the future so createNpmDirs isn't called on every build
await cli.createNpmDirs({
  packageJsonPath: NODE_SDK_PACKAGE_JSON_PATH,
  npmDir: NAPI_PLACEHOLDER_PACKAGES_DIR,
  cwd: ROOT_DIR,
});
await fs.copyFile(
  path.join(NAPI_PLACEHOLDER_PACKAGES_DIR, platformTriple, 'package.json'),
  path.join(BUILD_CORE_BINARY_DIR, 'package.json'),
);

// 5. Log the output.
console.log(`build -> _build/${NAPI_CONFIG.packageName}/ (snowflake-sdk-core-${platformTriple})`);
for (const file of await fs.readdir(BUILD_CORE_BINARY_DIR)) {
  console.log(`  ${file}`);
}
