import type * as CoreBinary from './binary-types.generated.js';
import { getLibcDetails } from './libc_details.js';

// Every `require` below MUST use a string literal: bundlers can only trace
// (and thus include) native dependencies when the specifier is statically
// analyzable. Do not refactor these into a computed name.
function getCore(): typeof CoreBinary {
  const { platform, arch } = process;

  switch (platform) {
    case 'darwin':
      switch (arch) {
        case 'arm64':
          return require('@snowflake/sdk-core-darwin-arm64');
        case 'x64':
          return require('@snowflake/sdk-core-darwin-x64');
      }
      break;
    case 'linux': {
      const isMusl = getLibcDetails().family === 'musl';
      switch (arch) {
        case 'arm64':
          return isMusl
            ? require('@snowflake/sdk-core-linux-arm64-musl')
            : require('@snowflake/sdk-core-linux-arm64-gnu');
        case 'x64':
          return isMusl
            ? require('@snowflake/sdk-core-linux-x64-musl')
            : require('@snowflake/sdk-core-linux-x64-gnu');
      }
      break;
    }
    case 'win32':
      switch (arch) {
        case 'arm64':
          return require('@snowflake/sdk-core-win32-arm64-msvc');
        case 'x64':
          return require('@snowflake/sdk-core-win32-x64-msvc');
        case 'ia32':
          return require('@snowflake/sdk-core-win32-ia32-msvc');
      }
      break;
  }

  throw new Error(`Unsupported platform for @snowflake/sdk-core: ${platform}-${arch}.`);
}

const core = getCore();

// Users get these from the package exactly as is, under the same names.
// A change to them in the native core changes the public API of the package.
export const getFipsStatus = core.getFipsStatus;
export type FipsStatus = CoreBinary.FipsStatus;
export type QueryStatus = CoreBinary.QueryStatus;
export type StatementStatus = CoreBinary.StatementStatus;

// Everything below is only for use inside the driver.
// Names must start with `Core` or `core`, so they are not confused with the public names above.
export const CoreConnection = core.Connection;
export const CoreStatement = core.Statement;
export const coreInitBridge = core.initBridge;
export const CoreColumn = core.Column;
export const coreFormatSnowflakeDate = core.formatSnowflakeDate;
export const CoreSessionState = core.SessionState;
export const CoreQueryBindingFormat = core.QueryBindingFormat;
export const coreSetCustomCredentialManager = core.setCustomCredentialManager;
export type CustomCredentialManager = CoreBinary.CustomCredentialManager;
export const coreLogEvent = core.logEvent;
export const coreSetLogLevel = core.setLogLevel;

export type CoreConnectionInstance = InstanceType<typeof CoreConnection>;
export type CoreStatementInstance = InstanceType<typeof CoreStatement>;
export type CoreColumnInstance = InstanceType<typeof CoreColumn>;
export type CoreSessionStateInstance = InstanceType<typeof CoreSessionState>;
export type CoreQueryBindings = CoreBinary.QueryBindings;
export type CoreKnownSessionParameters = CoreBinary.KnownSessionParameters;
export type ConnectionTokenInfo = CoreBinary.ConnectionTokenInfo;
export type CoreLogEvent = CoreBinary.LogEvent;
