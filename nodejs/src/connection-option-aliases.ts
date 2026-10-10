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
  privateKey: 'private_key',
  privateKeyPath: 'private_key_file',
  privateKeyPass: 'private_key_password',
  workloadIdentityProvider: 'workload_identity_provider',
  workloadIdentityAzureClientId: 'workload_identity_azure_client_id',
  workloadIdentityAzureEntraIdResource: 'workload_identity_entra_resource',
  clientStoreTemporaryCredential: 'client_store_temporary_credential',
  database: 'database',
  schema: 'schema',
  warehouse: 'warehouse',
  role: 'role',
  // Core's Node.js alias resolves this spelling to login_timeout (BD#91).
  retryTimeout: 'retryTimeout',
  // Core's Node.js alias resolves this spelling to authentication_timeout (BD#59).
  browserActionTimeout: 'browserActionTimeout',
  useEnvProxy: 'use_proxy_env',
  port: 'port',
  protocol: 'protocol',
  queryTag: 'query_tag',
  application: 'application',
  clientSessionKeepAlive: 'CLIENT_SESSION_KEEP_ALIVE',
  clientSessionKeepAliveHeartbeatFrequency: 'CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY',
  serverSessionKeepAlive: 'server_session_keep_alive',
  certRevocationCheckMode: 'crl_check_mode',
  crlAllowCertificatesWithoutCrlURL: 'crl_allow_certificates_without_crl_url',
  crlInMemoryCache: 'crl_enable_memory_caching',
  crlOnDiskCache: 'crl_enable_disk_caching',
};

export function toCoreConnectionOptions(options: Record<string, unknown>): Record<string, string> {
  const normalized: Record<string, string> = {};
  for (const [key, value] of Object.entries(options)) {
    if (value === undefined) {
      continue;
    }
    if (key === 'browserActionTimeout' && typeof value === 'number' && Number.isFinite(value)) {
      // Milliseconds in, seconds out (BD#59).
      // TODO(SNOW-4263660): take seconds, or replace this option with authenticationTimeout.
      let seconds = Math.floor(value / 1000);
      if (value > 0 && seconds === 0) {
        seconds = 1;
      }
      normalized.browserActionTimeout = String(seconds);
      continue;
    }
    const sfCoreKey = CONNECTION_OPTION_ALIASES[key];
    if (sfCoreKey === undefined) {
      throw new Error(`Unknown connection option: ${key}`);
    }
    normalized[sfCoreKey] = String(value);
  }
  return normalized;
}
