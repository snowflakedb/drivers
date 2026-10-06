import type { ConnectionOptions } from './index.js';

const KEY_ALIASES = new Map([
  ['user', 'username'],
  ['private_key_file', 'privateKeyPath'],
]);

const SNAKE_TO_CAMEL_OVERRIDES = new Map([
  ['client_request_mfa_token', 'clientRequestMFAToken'],
  ['disable_saml_url_check', 'disableSamlURLCheck'],
  ['crl_allow_certificates_without_crl_url', 'crlAllowCertificatesWithoutCrlURL'],
]);

function toCamelCaseKey(key: string): string {
  if (key !== key.toLowerCase() || !key.includes('_')) {
    return key;
  }
  return key.replace(/_([a-z])/g, (_, char: string) => char.toUpperCase());
}

/**
 * Converts snake_case connection option keys, such as a parsed `connections.toml`
 * entry, into the camelCase keys of {@link ConnectionOptions}. `user` becomes
 * `username` and `private_key_file` becomes `privateKeyPath`. Keys that are not
 * lowercase snake_case pass through unchanged, and `undefined` values are dropped.
 *
 * When two input keys resolve to the same option, the one that comes first in
 * the object wins.
 */
export function normalizeConnectionOptions(options: Record<string, unknown>): ConnectionOptions {
  const normalized = new Map<string, unknown>();
  for (const [key, value] of Object.entries(options)) {
    if (value === undefined) {
      continue;
    }
    const target = KEY_ALIASES.get(key) ?? SNAKE_TO_CAMEL_OVERRIDES.get(key) ?? toCamelCaseKey(key);
    if (!normalized.has(target)) {
      normalized.set(target, value);
    }
  }
  return Object.fromEntries(normalized);
}
