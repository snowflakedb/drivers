import { execFile } from 'node:child_process';
import fs from 'node:fs';
import { createRequire } from 'node:module';
import { createConnection } from 'node:net';
import os from 'node:os';
import path from 'node:path';
import { promisify } from 'node:util';
import { expect, onTestFinished } from 'vitest';
import type { Connection, SnowflakeError } from '../../types/sdk-types.js';
import type { WiremockServer } from '../utils/wiremock/server.js';
import ErrorCode from '../../../src/constants/ErrorCode.js';
import { getTestParametersFromSameSource } from '../utils/getTestParameter.js';
import {
  connectAsyncWithErrorBD,
  destroyConnectionAsync,
  executeAsync,
  sleepAsync,
  snowflake,
} from '../utils/index.js';

export const execFileAsync = promisify(execFile);
const OKTA_USER_KEY = 'SNOWFLAKE_TEST_OKTA_USER';
const OKTA_PASSWORD_KEY = 'SNOWFLAKE_TEST_OKTA_PASSWORD';
const MFA_USER_KEY = 'SNOWFLAKE_TEST_MFA_USER';
const MFA_PASSWORD_KEY = 'SNOWFLAKE_TEST_MFA_PASSWORD';
const MFA_SEED_KEY = 'SNOWFLAKE_TEST_MFA_SEED';
const TOTP_GENERATOR_SCRIPT = '/externalbrowser/totpGenerator.js';
const TOTP_STEP_SECONDS = 30;
const MAX_TOTP_WINDOWS = 3;
const CHROMIUM_PORT = 9222;

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

export function requireMfaCredentials(): { user: string; password: string; totpSeed: string } {
  const credentials = getTestParametersFromSameSource([
    MFA_USER_KEY,
    MFA_PASSWORD_KEY,
    MFA_SEED_KEY,
  ]);
  if (!credentials) {
    throw new Error(
      `No parameters source defines ${MFA_USER_KEY}, ${MFA_PASSWORD_KEY}, and ${MFA_SEED_KEY}`,
    );
  }
  return {
    user: credentials[MFA_USER_KEY],
    password: credentials[MFA_PASSWORD_KEY],
    totpSeed: credentials[MFA_SEED_KEY],
  };
}

function totpWindowId(nowMs = Date.now()): number {
  return Math.floor(nowMs / 1000 / TOTP_STEP_SECONDS);
}

function secondsUntilNextWindow(): number {
  const remaining = TOTP_STEP_SECONDS - ((Date.now() / 1000) % TOTP_STEP_SECONDS);
  return remaining > 0 ? remaining : 0;
}

type TotpGeneratorClass = new (seed: string) => { generateTotp(): Promise<string> };

async function getCurrentTotpCode(seed: string): Promise<string> {
  // Required lazily: the generator and its speakeasy dependency exist only in the auth container.
  const TotpGenerator = createRequire(import.meta.url)(TOTP_GENERATOR_SCRIPT) as TotpGeneratorClass;
  return new TotpGenerator(seed).generateTotp();
}

function mfaStateDir(): string {
  const root = process.env.WORKSPACE_ROOT ?? process.env.WORKSPACE ?? os.tmpdir();
  const tag = process.env.BUILD_TAG || 'local';
  return path.join(root, '.ud-mfa-totp-state', tag);
}

function usedWindowMarkerPath(windowId: number): string {
  return path.join(mfaStateDir(), `ud-mfa-used-totp-window-${windowId}`);
}

function exhaustedFlagPath(): string {
  return path.join(mfaStateDir(), 'ud-mfa-connect-exhausted');
}

function isSharedMfaExhausted(): boolean {
  return fs.existsSync(exhaustedFlagPath());
}

function markSharedMfaExhausted(): void {
  fs.mkdirSync(mfaStateDir(), { recursive: true });
  fs.writeFileSync(exhaustedFlagPath(), '1\n');
}

function claimTotpWindow(windowId: number): boolean {
  fs.mkdirSync(mfaStateDir(), { recursive: true });
  try {
    // 'wx' creation is atomic, so exactly one process can claim a window.
    fs.closeSync(fs.openSync(usedWindowMarkerPath(windowId), 'wx'));
    return true;
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === 'EEXIST') {
      return false;
    }
    throw error;
  }
}

async function freshTotpCode(
  seed: string,
): Promise<{ code: string; windowId: number } | undefined> {
  const code = await getCurrentTotpCode(seed);
  const windowId = totpWindowId();
  if (!claimTotpWindow(windowId)) {
    return undefined;
  }
  return { code, windowId };
}

async function sleepToNextTotpWindow(): Promise<void> {
  const wait = secondsUntilNextWindow();
  if (wait > 0) {
    // The next TOTP step is a fixed 30s clock; there is nothing to poll.
    await sleepAsync((wait + 1) * 1000);
  }
}

async function sleepIfStillInWindow(windowId: number): Promise<void> {
  if (totpWindowId() === windowId) {
    await sleepToNextTotpWindow();
  }
}

function isTotpRetryableError(error: unknown): boolean {
  const message = error instanceof Error ? error.message : String(error);
  return message.includes('TOTP Invalid') || message.toLowerCase().includes('invalid passcode');
}

function isMfaLockoutError(error: unknown): boolean {
  const message = error instanceof Error ? error.message : String(error);
  return message.includes('394512') || message.toLowerCase().includes('too many failed mfa');
}

export async function acquireTotpPasscode(
  seed: string,
  skip: (note?: string) => never,
): Promise<string> {
  if (isSharedMfaExhausted()) {
    skip('Shared MFA account already exhausted TOTP retries in this run');
  }
  for (let advances = 0; advances < MAX_TOTP_WINDOWS; advances++) {
    const minted = await freshTotpCode(seed);
    if (minted) {
      return minted.code;
    }
    await sleepToNextTotpWindow();
  }
  throw new Error(`Could not mint a TOTP for an unused window after ${MAX_TOTP_WINDOWS} windows`);
}

export async function connectWithTotpRetry(
  options: Record<string, unknown>,
  totpSeed: string,
  extras: {
    skip: (note?: string) => never;
    passcodeInPassword?: boolean;
  },
): Promise<Connection> {
  const { passcodeInPassword = false, skip } = extras;
  if (isSharedMfaExhausted()) {
    skip('Shared MFA account already exhausted TOTP retries in this run');
  }

  const basePassword = options.password;
  if (typeof basePassword !== 'string') {
    throw new Error('connectWithTotpRetry requires options.password');
  }

  let lastError: unknown;
  let submits = 0;
  let advances = 0;
  while (submits < MAX_TOTP_WINDOWS) {
    const minted = await freshTotpCode(totpSeed);
    if (!minted) {
      if (advances >= MAX_TOTP_WINDOWS) {
        break;
      }
      await sleepToNextTotpWindow();
      advances += 1;
      continue;
    }

    submits += 1;
    const attemptOptions = passcodeInPassword
      ? { ...options, password: basePassword + minted.code, passcodeInPassword: true }
      : { ...options, passcode: minted.code };
    const connection = snowflake.createConnection(attemptOptions);
    try {
      await connectAsyncWithErrorBD(connection);
      return connection;
    } catch (error) {
      lastError = error;
      if (isMfaLockoutError(error)) {
        markSharedMfaExhausted();
        throw error;
      }
      if (!isTotpRetryableError(error)) {
        throw error;
      }
      if (submits < MAX_TOTP_WINDOWS) {
        await sleepIfStillInWindow(minted.windowId);
      }
    }
  }

  if (submits === 0) {
    throw new Error(`Could not mint a TOTP for an unused window after ${MAX_TOTP_WINDOWS} windows`);
  }
  markSharedMfaExhausted();
  throw new Error(
    `Failed to connect after ${submits} TOTP submits. Last error: ${String(lastError)}`,
  );
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

export async function provideBrowserCredentials(...scriptArgs: string[]): Promise<void> {
  await waitForChromium();
  await execFileAsync('node', ['/externalbrowser/provideBrowserCredentials.js', ...scriptArgs], {
    timeout: 90_000,
  });
}

export async function connectWithBrowserAutomation(
  connect: () => Promise<unknown>,
  provideCredentials: () => Promise<void>,
): Promise<void> {
  const [connectResult, browserResult] = await Promise.allSettled([
    connect(),
    provideCredentials(),
  ]);
  onTestFinished(cleanBrowserProcesses);
  if (connectResult.status === 'rejected') {
    throw connectResult.reason;
  }
  if (browserResult.status === 'rejected') {
    throw browserResult.reason;
  }
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
      name: expected.name,
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
