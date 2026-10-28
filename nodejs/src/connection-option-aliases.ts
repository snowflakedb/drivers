import { invalidParameterError } from './error.js';

// TODO: In future we should register an alias in core for each parameter. This map then becomes a
// set: the Node driver's whitelist of connection options, even when core
// accepts more.
//
// Maps every camelCase connection option this driver accepts (the
// snowflake-sdk `ConnectionOptions` shape) onto the snake_case key sf_core
// understands. A key absent from this map is rejected, so a typo or an
// unsupported option surfaces at construction time rather than being forwarded
// to the bridge and silently ignored.
const CONNECTION_OPTION_ALIASES: Record<string, string> = {
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
  oauthEnableSingleUseRefreshTokens: 'oauth_enable_single_use_refresh_tokens',
  privateKey: 'private_key',
  privateKeyPath: 'private_key_file',
  privateKeyPass: 'private_key_password',
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
};

const MILLISECOND_OPTIONS = new Set([
  'browserActionTimeout',
  'timeout',
  'crlDownloadTimeout',
  'crlCacheValidityTime',
]);

export function toCoreConnectionOptions(options: Record<string, unknown>): Record<string, string> {
  const normalized: Record<string, string> = {};
  for (const [key, value] of Object.entries(options)) {
    if (value === undefined) {
      continue;
    }
    const sfCoreKey = CONNECTION_OPTION_ALIASES[key];
    if (sfCoreKey === undefined) {
      throw invalidParameterError(`Unknown connection option: ${key}`);
    }
    if (MILLISECOND_OPTIONS.has(key) && typeof value === 'number' && Number.isFinite(value)) {
      // Milliseconds in, seconds out (BD#59, BD#122, BD#123).
      // TODO(SNOW-4263660): take seconds, or replace browserActionTimeout with authenticationTimeout.
      let seconds = Math.floor(value / 1000);
      if (value > 0 && seconds === 0) {
        seconds = 1;
      }
      normalized[sfCoreKey] = String(seconds);
      continue;
    }
    normalized[sfCoreKey] = String(value);
  }
  applyNodeTokenOverrides(normalized);
  return normalized;
}

function applyNodeTokenOverrides(normalized: Record<string, string>): void {
  if (normalized.token) {
    // Backward compatibility. An inline token wins over tokenFilePath
    // (sf_core prefers the file).
    delete normalized.token_file_path;
  } else if (
    // Backward compatibility with the old Node driver: unlike other drivers,
    // it accepted PROGRAMMATIC_ACCESS_TOKEN with only a password and sent that
    // value as TOKEN, not also as password. sf_core requires token or
    // token_file_path, but never validates the value, so its errors and logs
    // remain accurate.
    normalized.authenticator?.toUpperCase() === 'PROGRAMMATIC_ACCESS_TOKEN' &&
    !normalized.token_file_path &&
    normalized.password
  ) {
    normalized.token = normalized.password;
    delete normalized.password;
  }
}
