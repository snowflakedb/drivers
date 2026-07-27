import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const PROJECT_ROOT = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  '..',
  '..',
  '..',
  '..',
);
const PARAMETER_PATH = process.env.PARAMETER_PATH ?? path.join(PROJECT_ROOT, 'parameters.json');

// Values in parameters.json are strings, or arrays of strings representing
// multiline values (e.g. the PEM key in SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS).
type ParameterValue = string | string[];

let parametersFromFile: Record<string, ParameterValue> = {};
if (fs.existsSync(PARAMETER_PATH)) {
  try {
    const raw = JSON.parse(fs.readFileSync(PARAMETER_PATH, 'utf-8'));
    parametersFromFile = raw.testconnection ?? {};
  } catch (e) {
    throw new Error(`Failed to parse parameters file: ${PARAMETER_PATH}`);
  }
}

export default function getTestParameter(key: string): string | undefined {
  const value = parametersFromFile[key];
  if (Array.isArray(value)) {
    return value.join('\n');
  }
  return value ?? process.env[key];
}
