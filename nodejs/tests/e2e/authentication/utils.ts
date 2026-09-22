import { execFile } from 'node:child_process';
import { createConnection } from 'node:net';
import { promisify } from 'node:util';
import { getTestParametersFromSameSource } from '../utils/getTestParameter.js';

const execFileAsync = promisify(execFile);
const OKTA_USER_KEY = 'SNOWFLAKE_TEST_OKTA_USER';
const OKTA_PASSWORD_KEY = 'SNOWFLAKE_TEST_OKTA_PASSWORD';
const CHROMIUM_PORT = 9222;

/**
 * Auth tests run only inside the external-browser Docker image
 * (`tests/docker/external-browser/Dockerfile`), which bundles Chromium and the
 * Playwright helpers that drive the OAuth, PAT, and MFA/TOTP flows.
 *
 * Run them with `tests/auth/run_auth_browser_local.sh` (builds the image) or, in
 * CI, `tests/auth/run_auth_browser.sh`. If the next argument is not `universal`
 * or `reference`, mode stays `universal` and that argument is a vitest arg.
 * Paths are relative to `nodejs/` and default to `tests/e2e/authentication/`:
 *
 * - `... nodejs` — new driver, full auth suite
 * - `... nodejs tests/e2e/authentication/oauth.test.ts -t "should authenticate"` — new driver, one test
 * - `... nodejs reference` — old driver, full auth suite
 * - `... nodejs reference tests/e2e/authentication/oauth.test.ts -t "should authenticate"` — old driver, one test
 *
 * Those scripts set `SF_TEST_HEADLESS_BROWSER=true` in the container; a plain
 * `npm run test:e2e` leaves the flag unset, so these tests are skipped.
 *
 * TODO(SNOW-3996212): fail the wrapper when this skip leaves zero tests executed (#1786).
 */
export const NOT_IN_AUTH_TEST_CONTAINER = process.env.SF_TEST_HEADLESS_BROWSER !== 'true';

export function requireOktaCredentials(): { user: string; password: string } {
  const credentials = getTestParametersFromSameSource([OKTA_USER_KEY, OKTA_PASSWORD_KEY]);
  if (!credentials) {
    throw new Error(`No parameters source defines both ${OKTA_USER_KEY} and ${OKTA_PASSWORD_KEY}`);
  }
  return {
    user: credentials[OKTA_USER_KEY],
    password: credentials[OKTA_PASSWORD_KEY],
  };
}

export function canConnectToChromium(): Promise<boolean> {
  return new Promise((resolve) => {
    const socket = createConnection({ host: '127.0.0.1', port: CHROMIUM_PORT });
    let completed = false;

    const complete = (connected: boolean) => {
      if (completed) {
        return;
      }
      completed = true;
      socket.destroy();
      resolve(connected);
    };

    socket.once('connect', () => complete(true));
    socket.once('error', () => complete(false));
    socket.setTimeout(500, () => complete(false));
  });
}

export async function waitForChromium(timeoutMs = 60_000): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (await canConnectToChromium()) {
      return;
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error(`Chromium did not start on port ${CHROMIUM_PORT} within ${timeoutMs} ms`);
}

export async function cleanBrowserProcesses(): Promise<void> {
  await execFileAsync('node', ['/externalbrowser/cleanBrowserProcesses.js'], {
    timeout: 15_000,
  });
}
