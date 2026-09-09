import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const dir = path.dirname(fileURLToPath(import.meta.url));
const PROJECT_ROOT = path.resolve(dir, '../../../..');
const PARAMETER_PATH = process.env.PARAMETER_PATH ?? path.join(PROJECT_ROOT, 'parameters.json');

const NODEJS_SECTION = 'testconnection-nodejs';
const BASE_SECTION = 'testconnection';
const sections = loadSections();
const nodejsAndBase = sections.filter(
  (section) => section.name === NODEJS_SECTION || section.name === BASE_SECTION,
);

type Section = { name: string; values: Record<string, unknown> };

/**
 * Read a test parameter. First defined value wins:
 *   1. `testconnection-nodejs` in the JSON file (Node overrides)
 *   2. `testconnection` in the same file (shared default)
 *   3. `process.env[key]` (last resort)
 *
 * The JSON path is `PARAMETER_PATH`, default `parameters.json` at the repo
 * root. Auth-browser Jenkins writes `parameters_preprod.json` and sets
 * `PARAMETER_PATH` to that file — it does not inject every key via env.
 * Decode once with `npm run creds:decode` (nodejs/) or
 * `./scripts/decode_secrets.sh` (repo root). Empty strings count as missing.
 *
 * Default / omitted / false stays non-throwing so module-load callers such as
 * `baseConnectionOptions` can import without parameters present.
 */
export default function getTestParameter(key: string, required: true): string;
export default function getTestParameter(key: string, required?: false): string | undefined;
export default function getTestParameter(key: string, required?: boolean): string | undefined {
  let value: string | undefined;
  for (const section of nodejsAndBase) {
    value = readValue(section.values, key);
    if (value !== undefined) {
      break;
    }
  }
  if (value === undefined) {
    value = process.env[key];
  }
  if (required && !value) {
    throw new Error(`Required test parameter is missing: ${key}`);
  }
  return value;
}

/**
 * Reads keys that only make sense together, such as an IdP user and its
 * password, from a single source. Walks every loaded JSON section
 * (`testconnection-nodejs`, `testconnection`, then remaining
 * `testconnection-*`) and then `process.env`. Returns `undefined` when no
 * single source supplies the whole set.
 *
 * Same `PARAMETER_PATH` / `parameters_preprod.json` story as
 * `getTestParameter`; Jenkins does not inject every key via env.
 */
export function getTestParametersFromSameSource<const K extends string>(
  keys: readonly K[],
): Record<K, string> | undefined {
  for (const section of sections) {
    const resolved = {} as Record<K, string>;
    for (const key of keys) {
      const value = readValue(section.values, key);
      if (value === undefined) {
        break;
      }
      resolved[key] = value;
    }
    if (Object.keys(resolved).length === keys.length) {
      return resolved;
    }
  }

  const fromEnv = {} as Record<K, string>;
  for (const key of keys) {
    const value = process.env[key];
    if (!value) {
      return undefined;
    }
    fromEnv[key] = value;
  }
  return fromEnv;
}

function isSection(section: { name: string; values: unknown }): section is Section {
  return (
    section.values !== null && typeof section.values === 'object' && !Array.isArray(section.values)
  );
}

// Arrays join with newlines (PEM); empty string is missing.
function readValue(section: Record<string, unknown>, key: string): string | undefined {
  const value = section[key];
  if (typeof value === 'string') {
    return value.length > 0 ? value : undefined;
  }
  if (Array.isArray(value) && value.every((part) => typeof part === 'string')) {
    return value.length > 0 ? value.join('\n') : undefined;
  }
  return undefined;
}

function loadSections(): Section[] {
  if (!fs.existsSync(PARAMETER_PATH)) {
    return [];
  }
  try {
    const raw = JSON.parse(fs.readFileSync(PARAMETER_PATH, 'utf-8')) as Record<string, unknown>;
    const otherSections = Object.keys(raw)
      .filter((name) => name.startsWith(`${BASE_SECTION}-`) && name !== NODEJS_SECTION)
      .sort();
    return [NODEJS_SECTION, BASE_SECTION, ...otherSections]
      .map((name) => ({ name, values: raw[name] }))
      .filter(isSection);
  } catch {
    throw new Error(`Failed to parse parameters file: ${PARAMETER_PATH}`);
  }
}
