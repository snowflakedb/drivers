import * as fs from 'node:fs/promises';
import * as path from 'node:path';
import * as url from 'node:url';

const __dirname = path.dirname(url.fileURLToPath(import.meta.url));

export const ROOT_DIR = path.resolve(__dirname, '..');
export const NODE_SDK_PACKAGE_JSON_PATH = path.join(ROOT_DIR, 'package.json');
export const NODE_SDK_PACKAGE = JSON.parse(await fs.readFile(NODE_SDK_PACKAGE_JSON_PATH, 'utf8'));
export const BUILD_DIR = path.join(ROOT_DIR, '_build');
