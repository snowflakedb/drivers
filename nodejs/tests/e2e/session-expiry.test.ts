import { afterAll, beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { createLiveConnection } from './utils/fixtures.js';
import { executeAsync } from './utils/index.js';
import {
  loginSuccess,
  queryRequestFail,
  tokenRequestFail,
  WiremockServer,
} from './utils/wiremock/index.js';

const SESSION_TOKEN_EXPIRED = '390112';
const MASTER_TOKEN_EXPIRED = '390114';

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
});
