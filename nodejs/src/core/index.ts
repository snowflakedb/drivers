import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);

// NOTE:
// This is just placeholder file to test if binary compiled correctly
// There will be a separate PR implementing proper platform-specific loading logic
const core =
  require('snowflake-sdk-core-darwin-arm64') as typeof import('snowflake-sdk-core-darwin-arm64');

// oxlint-disable-next-line no-console
console.log('core', core.dummyTestEntrypoint());

export default core;
