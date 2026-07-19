import { createRequire } from 'node:module';
import type * as CoreBinary from './binary-types.generated.js';

const require = createRequire(import.meta.url);

// NOTE:
// This is just placeholder file to test if binary compiled correctly
// There will be a separate PR implementing proper platform-specific if/else loading logic
function getCore(): typeof CoreBinary {
  return require('snowflake-sdk-core-darwin-arm64');
}

const core = getCore();

// oxlint-disable-next-line no-console
console.log('core', core.dummyTestEntrypoint());

export default core;
