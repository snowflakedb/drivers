import { createRequire } from 'node:module';
import type * as NativeCore from '../../_build/snowflake-sdk-core/index.d.ts';

const require = createRequire(import.meta.url);

// NOTE:
// This is just placeholder file to test if binary compiled correctly
// There will be a separate PR implementing proper platform-specific loading logic
const core =
  require('../../_build/snowflake-sdk-core/snowflake-sdk-core.darwin-arm64.node') as typeof NativeCore;

// oxlint-disable-next-line no-console
console.log('core', core.dummyTestEntrypoint());

export default core;
