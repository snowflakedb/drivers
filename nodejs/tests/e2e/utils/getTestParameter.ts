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

let parametersFromFile: Record<string, string> = {};
if (fs.existsSync(PARAMETER_PATH)) {
  try {
    const raw = JSON.parse(fs.readFileSync(PARAMETER_PATH, 'utf-8'));
    parametersFromFile = raw.testconnection ?? {};
  } catch (e) {
    throw new Error(`Failed to parse parameters file: ${PARAMETER_PATH}`);
  }
}

export default function getTestParameter(key: string): string | undefined {
  return parametersFromFile[key] ?? process.env[key];
}
