// Maps every camelCase connection option this driver accepts (the
// snowflake-sdk `ConnectionOptions` shape) onto the snake_case key sf_core
// understands. A key absent from this map is rejected, so a typo or an
// unsupported option surfaces at construction time rather than being forwarded
// to the bridge and silently ignored.
const CONNECTION_OPTION_ALIASES: Record<string, string> = {
  account: 'account',
  host: 'host',
  username: 'user',
  password: 'password',
  authenticator: 'authenticator',
  token: 'token',
  privateKey: 'private_key',
  privateKeyPass: 'private_key_password',
  workloadIdentityProvider: 'workload_identity_provider',
  clientStoreTemporaryCredential: 'client_store_temporary_credential',
  database: 'database',
  schema: 'schema',
  warehouse: 'warehouse',
  role: 'role',
  useEnvProxy: 'use_proxy_env',
  port: 'port',
  protocol: 'protocol',
};

export function normalizeConnectionOptions(
  options: Record<string, unknown>,
): Record<string, string> {
  const normalized: Record<string, string> = {};
  for (const [key, value] of Object.entries(options)) {
    if (value === undefined) {
      continue;
    }
    if (key === 'browserActionTimeout') {
      // The Node SDK takes milliseconds; sf_core takes authentication_timeout in seconds (BD#48).
      if (typeof value !== 'number' || !Number.isFinite(value) || value <= 0) {
        throw new Error('browserActionTimeout must be a positive number');
      }
      normalized.authentication_timeout = String(Math.floor(value / 1000));
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
