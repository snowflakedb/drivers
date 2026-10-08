import { afterAll, beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { createLiveConnection } from './utils/fixtures.js';
import { executeAsync, isRunningNewDriverWithBD } from './utils/index.js';
import {
  loginSuccess,
  logoutSuccess,
  queryRequestFail,
  tokenRequestConnectionReset,
  tokenRequestFail,
  WiremockServer,
} from './utils/wiremock/index.js';

const SESSION_TOKEN_EXPIRED = '390112';
const MASTER_TOKEN_EXPIRED = '390114';
const SESSION_GONE = '390111';

describe('Session Expiry', () => {
  let wiremock: WiremockServer;

  beforeAll(async () => {
    wiremock = await WiremockServer.spawn();
  });

  beforeEach(async () => {
    await wiremock.reset();
  });

  afterAll(async () => {
    await wiremock.destroy();
  });

  it('should report a connection as down once the master token has expired', async () => {
    await wiremock.stub(loginSuccess());
    await wiremock.stub(queryRequestFail(SESSION_TOKEN_EXPIRED, 'Session token expired.'));
    await wiremock.stub(tokenRequestFail(MASTER_TOKEN_EXPIRED, 'Authentication token expired.'));

    const connection = await createLiveConnection(wiremock.connectionOptions, false);
    expect(connection.isUp()).toBe(true);

    await expect(executeAsync(connection, 'select 1')).rejects.toMatchObject({
      error: expect.any(Error),
    });
    expect(connection.isUp()).toBe(false);
  });

  it('should report a connection as down once the server refuses to renew the session', async () => {
    await wiremock.stub(loginSuccess());
    await wiremock.stub(queryRequestFail(SESSION_TOKEN_EXPIRED, 'Session token expired.'));
    await wiremock.stub(tokenRequestFail(SESSION_GONE, 'Session no longer exists.'));

    const connection = await createLiveConnection(wiremock.connectionOptions, false);
    expect(connection.isUp()).toBe(true);

    await expect(executeAsync(connection, 'select 1')).rejects.toMatchObject({
      error: expect.any(Error),
    });
    expect(connection.isUp()).toBe(false);
  });

  // Against a stub that always answers SESSION_TOKEN_EXPIRED the old driver renews and re-sends
  // the query forever, so there is no old-driver outcome to assert.
  it.skipIf(!isRunningNewDriverWithBD('BD#80'))(
    'should report NetworkError when the session renewal cannot reach Snowflake',
    async () => {
      await wiremock.stub(loginSuccess());
      await wiremock.stub(queryRequestFail(SESSION_TOKEN_EXPIRED, 'Session token expired.'));
      await wiremock.stub(tokenRequestConnectionReset());
      await wiremock.stub(logoutSuccess());

      const connection = await createLiveConnection(wiremock.connectionOptions);

      await expect(executeAsync(connection, 'select 1')).rejects.toMatchObject({
        error: {
          name: 'NetworkError',
          code: 401001,
          message: 'Network error. Could not reach Snowflake.',
        },
      });
      expect(connection.isUp()).toBe(true);
    },
  );
});
