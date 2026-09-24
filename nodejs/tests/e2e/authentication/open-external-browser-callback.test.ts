import { request as httpRequest } from 'node:http';
import { afterAll, beforeAll, beforeEach, describe, expect, it, onTestFinished } from 'vitest';
import type { Connection, SnowflakeError } from '../../types/sdk-types.js';
import {
  connectAsyncWithErrorBD,
  destroyConnectionAsync,
  IS_RUNNING_FOR_OLD_DRIVER,
  snowflake,
} from '../utils/index.js';
import {
  authenticatorRequestSuccess,
  loginSuccess,
  logoutSuccess,
  WiremockServer,
} from '../utils/wiremock/index.js';

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
    await wiremock.stub(logoutSuccess());
  });

  // The legacy driver HTTP-GETs the SSO URL (the IdP host) as part of login.
  // This fixture only stubs Snowflake session routes, so SSO is asserted on the new driver.
  it.skipIf(IS_RUNNING_FOR_OLD_DRIVER)(
    'should open EXTERNALBROWSER SSO through the callback and complete login',
    async () => {
      await wiremock.stub(authenticatorRequestSuccess());
      await wiremock.stub(loginSuccess());

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
      onTestFinished(async () => {
        await destroyConnectionAsync(connection);
      });
      await Promise.all([connection.connectAsync(), loopback]);

      expect(seen).toHaveLength(1);
      expect(seen[0]).toMatch(/^https:\/\/idp\.snowflake\.com\/sso\?/);
    },
  );

  it('should fail connect when openExternalBrowserCallback throws', async () => {
    await wiremock.stub(authenticatorRequestSuccess());

    const connection = snowflake.createConnection({
      account: 'testaccount',
      username: 'alice',
      authenticator: 'EXTERNALBROWSER',
      ...wiremock.connectionOptions,
      ...(IS_RUNNING_FOR_OLD_DRIVER ? { disableConsoleLogin: false } : {}),
      openExternalBrowserCallback: () => {
        throw new Error("you don't have a browser");
      },
    }) as Connection;
    onTestFinished(async () => {
      if (connection.isUp()) {
        await destroyConnectionAsync(connection);
      }
    });

    await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject({
      message: expect.stringContaining("you don't have a browser"),
    } satisfies Partial<SnowflakeError>);
  });
});
