import type { ConfigureOptions } from './global-config.js';
import type { ConnectionOptions } from './public-types/index.js';

// The Connection constructor handles these options itself, so they never reach nodejs_bridge.
type WrapperOnlyConnectionOption =
  | 'rowMode'
  | 'fetchAsString'
  | 'jsTreatIntegerAsBigInt'
  | 'representNullAsStringNull'
  | 'rowStreamHighWaterMark'
  | 'streamResult'
  | 'arrayBindingThreshold'
  | 'resultPrefetch'
  | 'openExternalBrowserCallback'
  | 'browserResponseRenderer';

// sf_core reads these snowflake.configure() options as connection options.
type GlobalConfigConnectionOption =
  | 'useEnvProxy'
  | 'crlDownloadTimeout'
  | 'crlDownloadMaxSize'
  | 'crlCacheValidityTime'
  | 'crlCacheDir';

export type CoreConnectionOptionsInput = Omit<ConnectionOptions, WrapperOnlyConnectionOption> &
  Pick<ConfigureOptions, GlobalConfigConnectionOption>;

// Keys keep the caller's spelling. sf_core resolves each Node.js option name,
// converts millisecond options to seconds, and warns about unknown options.
export function toCoreConnectionOptions(
  options: CoreConnectionOptionsInput,
): Record<string, string> {
  const normalized: Record<string, string> = {};
  for (const [key, value] of Object.entries(options)) {
    // The types do not allow null, but plain JavaScript callers can pass it.
    // Like the old driver, null means "not set".
    if (value === undefined || value === null) {
      continue;
    }
    normalized[key] = String(value);
  }
  applyOldDriverTokenRules(normalized);
  return normalized;
}

// TODO(SNOW-4517625): move these rules into sf_core.
// sf_core reads `token` and `tokenFilePath` with different rules than the
// old Node driver. This keeps the old Node behavior:
// - An inline `token` wins over `tokenFilePath`. sf_core reads the file when
//   both are set.
// - With PROGRAMMATIC_ACCESS_TOKEN, the old driver accepted the token in
//   `password` alone and sent it as TOKEN. sf_core needs `token` or
//   `tokenFilePath`, so the value moves from `password` to `token`.
function applyOldDriverTokenRules(normalized: Record<string, string>): void {
  if (normalized.token) {
    delete normalized.tokenFilePath;
  } else if (
    normalized.authenticator?.toUpperCase() === 'PROGRAMMATIC_ACCESS_TOKEN' &&
    !normalized.tokenFilePath &&
    normalized.password
  ) {
    normalized.token = normalized.password;
    delete normalized.password;
  }
}
