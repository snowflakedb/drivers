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
  privateKey: 'private_key',
  privateKeyPass: 'private_key_password',
  database: 'database',
  schema: 'schema',
  warehouse: 'warehouse',
  role: 'role',
  useEnvProxy: 'use_proxy_env',
};

export function normalizeConnectionOptions(
  options: Record<string, string>,
): Record<string, string> {
  const normalized: Record<string, string> = {};
  for (const [key, value] of Object.entries(options)) {
    const sfCoreKey = CONNECTION_OPTION_ALIASES[key];
    if (sfCoreKey === undefined) {
      throw new Error(`Unknown connection option: ${key}`);
    }
    normalized[sfCoreKey] = value;
  }
  return normalized;
}
