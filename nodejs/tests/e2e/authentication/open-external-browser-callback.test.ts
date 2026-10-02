import { request as httpRequest } from 'node:http';
import { afterAll, beforeAll, beforeEach, describe, expect, it } from 'vitest';
import type { Connection, SnowflakeError } from '../../types/sdk-types.js';
import {
  connectAsyncWithErrorBD,
  IS_RUNNING_FOR_OLD_DRIVER,
  isRunningNewDriverWithBD,
  snowflake,
} from '../utils/index.js';
import {
  authenticatorRequestSuccess,
  loginSuccess,
  logoutSuccess,
  WiremockServer,
} from '../utils/wiremock/index.js';
import { destroyConnectionAfterTest } from './utils.js';

function completeSsoLoopback(loginUrl: string): Promise<void> {
  const callbackPort = new URL(loginUrl).searchParams.get('browser_mode_redirect_port');
  if (!callbackPort) {
    throw new Error(`SSO URL missing browser_mode_redirect_port: ${loginUrl}`);
  }
  const target = new URL(`http://127.0.0.1:${callbackPort}/?token=test-token`);
  return new Promise((resolve, reject) => {
    const req = httpRequest(
      {
        hostname: target.hostname,
        port: target.port,
        path: `${target.pathname}${target.search}`,
        method: 'GET',
      },
      (res) => {
        res.resume();
        res.on('end', () => resolve());
      },
    );
    req.once('error', reject);
    req.end();
  });
}

// TODO(SNOW-3996212): merge this file with the EXTERNALBROWSER e2e coverage
// from snowflakedb/drivers#1759.
describe('openExternalBrowserCallback', () => {
  let wiremock: WiremockServer;

  beforeAll(async () => {
    wiremock = await WiremockServer.spawn();
  });

  afterAll(async () => {
    await wiremock.destroy();
  });

  beforeEach(async () => {
    await wiremock.reset();
  });

  // The legacy driver HTTP-GETs the SSO URL (the IdP host) as part of login.
  // This fixture only stubs Snowflake session routes, so SSO is asserted on the new driver.
  it.skipIf(IS_RUNNING_FOR_OLD_DRIVER)(
    'should open EXTERNALBROWSER SSO through the callback and complete login',
    async () => {
      await wiremock.stub(authenticatorRequestSuccess());
      await wiremock.stub(loginSuccess());
      await wiremock.stub(logoutSuccess());

      const seen: string[] = [];
      let resolveLoopback!: () => void;
      let rejectLoopback!: (reason: unknown) => void;
      const loopback = new Promise<void>((resolve, reject) => {
        resolveLoopback = resolve;
        rejectLoopback = reject;
      });
      const connection = snowflake.createConnection({
        account: 'testaccount',
        username: 'alice',
        authenticator: 'EXTERNALBROWSER',
        ...wiremock.connectionOptions,
        openExternalBrowserCallback: (url: string) => {
          seen.push(url);
          completeSsoLoopback(url).then(resolveLoopback, rejectLoopback);
        },
      }) as Connection;
      await Promise.all([connection.connectAsync(), loopback]);
      destroyConnectionAfterTest(connection);

      expect(seen).toHaveLength(1);
      expect(seen[0]).toMatch(/^https:\/\/idp\.snowflake\.com\/sso\?/);
    },
  );

  it('should fail connect when openExternalBrowserCallback throws', async () => {
    await wiremock.stub(authenticatorRequestSuccess());

    const callbackErrorMessage = "you don't have a browser";

    const connection = snowflake.createConnection({
      account: 'testaccount',
      username: 'alice',
      authenticator: 'EXTERNALBROWSER',
      ...wiremock.connectionOptions,
      ...(!isRunningNewDriverWithBD('BD#63') ? { disableConsoleLogin: false } : {}),
      openExternalBrowserCallback: () => {
        throw new Error(callbackErrorMessage);
      },
    }) as Connection;

    await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject({
      // The new driver appends the thrown error's stack, so the path is not part of the
      // pinned text. The message starts with this exact prefix.
      message: isRunningNewDriverWithBD('BD#67')
        ? expect.stringMatching(
            new RegExp(
              `^Failed to login: External browser SSO failed: Error: ${callbackErrorMessage}\n`,
            ),
          )
        : callbackErrorMessage,
    } satisfies Partial<SnowflakeError>);
  });
});
