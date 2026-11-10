import type { ConfigureOptions } from './global-config.js';
import type { ConnectionOptions } from './public-types/index.js';
import { logger } from './logging/index.js';

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

export type CoreConnectionOptionsInput = Omit<ConnectionOptions, WrapperOnlyConnectionOption> &
  Pick<
    ConfigureOptions,
    | 'useEnvProxy'
    | 'crlDownloadTimeout'
    | 'crlDownloadMaxSize'
    | 'crlCacheValidityTime'
    | 'crlCacheDir'
  >;

// TODO: In future we should register an alias in core for each parameter. This map then becomes a
// set: the Node driver's whitelist of connection options, even when core
// accepts more.
//
// Maps every camelCase connection option this driver accepts (the
// snowflake-sdk `ConnectionOptions` shape) onto the snake_case key sf_core
// understands. A key absent from this map is dropped - the driver
// ignores options it does not know and they are not forwarded to sf_core.
const CONNECTION_OPTION_ALIASES = {
  account: 'account',
  accessUrl: 'server_url',
  host: 'host',
  username: 'user',
  password: 'password',
  authenticator: 'authenticator',
  token: 'token',
  tokenFilePath: 'token_file_path',
  clientStoreTemporaryCredential: 'client_store_temporary_credential',
  oauthClientId: 'oauth_client_id',
  oauthClientSecret: 'oauth_client_secret',
  oauthRedirectUri: 'oauth_redirect_uri',
  oauthAuthorizationUrl: 'oauth_authorization_url',
  oauthTokenRequestUrl: 'oauth_token_request_url',
  oauthScope: 'oauth_scope',
  oauthEnableSingleUseRefreshTokens: 'oauth_enable_single_use_refresh_tokens',
  disableSamlURLCheck: 'disable_saml_url_check',
  privateKey: 'private_key',
  privateKeyPath: 'private_key_file',
  privateKeyPass: 'private_key_password',
  jwtTimeout: 'jwt_timeout',
  workloadIdentityProvider: 'workload_identity_provider',
  workloadIdentityAzureClientId: 'workload_identity_azure_client_id',
  workloadIdentityAzureEntraIdResource: 'workload_identity_entra_resource',
  workloadIdentityHost: 'wif_host',
  workloadIdentityImpersonationPath: 'workload_identity_impersonation_path',
  workloadIdentityAwsUseOutboundToken: 'workload_identity_aws_use_outbound_token',
  passcode: 'passcode',
  passcodeInPassword: 'passcodeInPassword',
  database: 'database',
  schema: 'schema',
  warehouse: 'warehouse',
  role: 'role',
  // Core's Node.js alias resolves this spelling to login_timeout (BD#91).
  retryTimeout: 'retryTimeout',
  // Core's Node.js alias resolves this spelling to authentication_timeout (BD#59).
  browserActionTimeout: 'browserActionTimeout',
  // Local EXTERNALBROWSER callback port. Core key `browser_redirect_port`.
  browserRedirectPort: 'browser_redirect_port',
  // Core's Node.js alias resolves this spelling to retry_timeout (BD#122).
  timeout: 'timeout',
  useEnvProxy: 'use_proxy_env',
  proxy: 'proxy',
  proxyHost: 'proxy_host',
  proxyPort: 'proxy_port',
  proxyUser: 'proxy_user',
  proxyPassword: 'proxy_password',
  proxyProtocol: 'proxy_scheme',
  noProxy: 'no_proxy',
  port: 'port',
  protocol: 'protocol',
  queryTag: 'query_tag',
  application: 'application',
  clientSessionKeepAlive: 'CLIENT_SESSION_KEEP_ALIVE',
  clientSessionKeepAliveHeartbeatFrequency: 'CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY',
  serverSessionKeepAlive: 'server_session_keep_alive',
  disableQueryContextCache: 'disable_query_context_cache',
  certRevocationCheckMode: 'crl_check_mode',
  crlAllowCertificatesWithoutCrlURL: 'crl_allow_certificates_without_crl_url',
  crlInMemoryCache: 'crl_enable_memory_caching',
  crlOnDiskCache: 'crl_enable_disk_caching',
  connectionName: 'connection_name',
  crlDownloadTimeout: 'crl_http_timeout',
  crlDownloadMaxSize: 'crl_max_download_size',
  crlCacheValidityTime: 'crl_validity_time',
  crlCacheDir: 'crl_cache_dir',
} satisfies Record<keyof CoreConnectionOptionsInput, string>;

const MILLISECOND_OPTIONS = new Set<keyof CoreConnectionOptionsInput>([
  'browserActionTimeout',
  'timeout',
  'crlDownloadTimeout',
  'crlCacheValidityTime',
]);

function isKnownOption(key: string): key is keyof CoreConnectionOptionsInput {
  return Object.hasOwn(CONNECTION_OPTION_ALIASES, key);
}

export function toCoreConnectionOptions(
  options: CoreConnectionOptionsInput,
): Record<string, string> {
  const normalized: Record<string, string> = {};

  for (const [key, value] of Object.entries(options)) {
    if (!isKnownOption(key)) {
      logger.warn(`Unknown option ${key} is ignored`);
      continue;
    }

    // The type does not allow null, but plain JavaScript callers can pass it.
    // Like the old driver, null means "not set".
    if (value === undefined || value === null) {
      continue;
    }

    const sfCoreKey = CONNECTION_OPTION_ALIASES[key];
    if (MILLISECOND_OPTIONS.has(key) && typeof value === 'number' && Number.isFinite(value)) {
      // Milliseconds in, seconds out (BD#59, BD#122, BD#123).
      // TODO(SNOW-4263660): take seconds, or replace browserActionTimeout with authenticationTimeout.
      let seconds = Math.floor(value / 1000);
      if (value > 0 && seconds === 0) {
        seconds = 1;
      }
      normalized[sfCoreKey] = String(seconds);
    } else {
      normalized[sfCoreKey] = String(value);
    }
  }

  applyOldDriverTokenRules(normalized);
  return normalized;
}

// sf_core reads `token` and `token_file_path` with different rules than the
// old Node driver. This keeps the old Node behavior:
// - An inline `token` wins over `tokenFilePath`. sf_core reads the file when
//   both are set.
// - With PROGRAMMATIC_ACCESS_TOKEN, the old driver accepted the token in
//   `password` alone and sent it as TOKEN. sf_core needs `token` or
//   `token_file_path`, so the value moves from `password` to `token`.
function applyOldDriverTokenRules(normalized: Record<string, string>): void {
  if (normalized.token) {
    delete normalized.token_file_path;
  } else if (
    normalized.authenticator?.toUpperCase() === 'PROGRAMMATIC_ACCESS_TOKEN' &&
    !normalized.token_file_path &&
    normalized.password
  ) {
    normalized.token = normalized.password;
    delete normalized.password;
  }
}
