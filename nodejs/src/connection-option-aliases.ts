// Maps legacy snowflake-sdk connection option names onto this driver's
// canonical snake_case names, so callers using the old SDK's camelCase
// option shape (e.g. `username`, `privateKey`, `privateKeyPass`) continue
// to work without sf_core or other language wrappers needing to know about
// Node-specific naming.
const CONNECTION_OPTION_ALIASES: Record<string, string> = {
  username: 'user',
  privateKey: 'private_key',
  privateKeyPass: 'private_key_password',
};

export function normalizeConnectionOptions(
  options: Record<string, string>,
): Record<string, string> {
  const normalized: Record<string, string> = {};
  for (const [key, value] of Object.entries(options)) {
    normalized[CONNECTION_OPTION_ALIASES[key] ?? key] = value;
  }
  return normalized;
}
