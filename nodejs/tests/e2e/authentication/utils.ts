import { execFile } from 'node:child_process';
import { createRequire } from 'node:module';
import { createConnection } from 'node:net';
import { promisify } from 'node:util';
import { expect, onTestFinished } from 'vitest';
import type { Connection } from '../../types/sdk-types.js';
import type { SnowflakeError } from '../../types/sdk-types.js';
import type { WiremockServer } from '../utils/wiremock/server.js';
import ErrorCode from '../../../src/constants/ErrorCode.js';
import { getTestParametersFromSameSource } from '../utils/getTestParameter.js';
import { destroyConnectionAsync, executeAsync, sleepAsync } from '../utils/index.js';

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

export function destroyConnectionAfterTest(connection: Connection): void {
  onTestFinished(async () => {
    await destroyConnectionAsync(connection);
  });
}

// Old getSSOURL concatenates rest.HTTPS_PROTOCOL and host. The published package does not
// export that field, so this reaches the CJS singleton. snowflake-sdk-old is always
// installed; the new driver never reads this object, so the write is unused when that
// driver is under test.
export function patchOldDriverHttpsProtocol(value: string): string {
  const { rest } = createRequire(import.meta.url)(
    'snowflake-sdk-old/dist/lib/global_config.js',
  ) as { rest: { HTTPS_PROTOCOL: string } };
  const previous = rest.HTTPS_PROTOCOL;
  rest.HTTPS_PROTOCOL = value;
  return previous;
}

export const TERMINATED_QUERY = {
  name: 'ClientError',
  code: ErrorCode.ERR_CONN_REQUEST_STATUS_DISCONNECTED,
  message: 'Unable to perform operation using terminated connection.',
} satisfies SnowflakeError;

export const NEVER_ESTABLISHED_QUERY = {
  name: 'ClientError',
  code: ErrorCode.ERR_CONN_REQUEST_STATUS_PRISTINE,
  message: 'Unable to perform operation because a connection was never established.',
} satisfies SnowflakeError;

// A login the server rejects leaves the connection terminated on both drivers. A failure with
// no session-rejecting response differs between them (BD#61) and is asserted at its call site.
export async function expectConnectionNotUp(
  connection: Connection,
  expected: SnowflakeError = TERMINATED_QUERY,
): Promise<void> {
  expect(connection.isUp()).toBe(false);
  await expect(executeAsync(connection, 'SELECT 1')).rejects.toMatchObject({
    error: {
      code: expected.code,
      message: expected.message,
    },
  });
}

// Plays the browser's part of an external-browser login. In the real flow:
//   1. The driver listens locally and sends the port as BROWSER_MODE_REDIRECT_PORT in
//      /session/authenticator-request; the server replies with an SSO URL.
//   2. After sign-in, the browser is redirected to http://localhost:<port>/?token=<token>.
// Tests have no real browser or sign-in page, so this helper waits `delayMs` and then makes
// the step 2 request to that port itself.
//
// Call it from openExternalBrowserCallback, which runs after the authenticator-request is sent.
export async function deliverBrowserCallbackAfter(
  wiremock: WiremockServer,
  token: string,
  delayMs: number,
): Promise<void> {
  const requests = await wiremock.findRequests('/session/authenticator-request.*');
  const body = requests[0]?.body;
  const parsed = body
    ? (JSON.parse(body) as { data?: { BROWSER_MODE_REDIRECT_PORT?: string } })
    : undefined;
  const port = Number(parsed?.data?.BROWSER_MODE_REDIRECT_PORT);
  if (!Number.isInteger(port) || port <= 0) {
    throw new Error(
      `BROWSER_MODE_REDIRECT_PORT missing in authenticator-request: ${body ?? 'no request'}`,
    );
  }
  await sleepAsync(delayMs);
  await deliverTokenToCallbackPort(port, token);
}

function deliverTokenToCallbackPort(port: number, token: string): Promise<void> {
  return new Promise((resolve, reject) => {
    const socket = createConnection({ host: '127.0.0.1', port });
    const fail = (error: Error) => {
      socket.destroy();
      reject(error);
    };
    socket.once('error', fail);
    socket.setTimeout(5_000, () => {
      fail(new Error(`Timed out delivering browser callback to port ${port}`));
    });
    socket.once('connect', () => {
      socket.write(`GET /?token=${token} HTTP/1.1\r\nHost: localhost\r\n\r\n`);
    });
    socket.once('data', () => {
      socket.end();
      resolve();
    });
  });
}
