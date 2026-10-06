import { afterAll, beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { createConnection } from '../utils/fixtures.js';
import { destroyConnectionAsync } from '../utils/index.js';
import { loginSuccess, logoutSuccess, WiremockServer } from '../utils/wiremock/index.js';

describe('serverSessionKeepAlive', () => {
  let wiremock: WiremockServer;

  beforeAll(async () => {
    wiremock = await WiremockServer.spawn();
  });

  beforeEach(async () => {
    await wiremock.reset();
    await wiremock.stub([loginSuccess(), logoutSuccess()]);
  });

  afterAll(async () => {
    await wiremock.destroy();
  });

  async function sessionDeleteCount(): Promise<number> {
    const requests = await wiremock.findRequests('/session');
    return requests.filter((request) => request.url.includes('delete=true')).length;
  }

  it('should not send a session delete when serverSessionKeepAlive is true', async () => {
    const connection = createConnection({
      ...wiremock.connectionOptions,
      serverSessionKeepAlive: true,
    });

    await connection.connectAsync();
    await destroyConnectionAsync(connection);

    expect(await sessionDeleteCount()).toBe(0);
  });

  it('should send a session delete when serverSessionKeepAlive is false', async () => {
    const connection = createConnection({
      ...wiremock.connectionOptions,
      serverSessionKeepAlive: false,
    });

    await connection.connectAsync();
    await destroyConnectionAsync(connection);

    expect(await sessionDeleteCount()).toBe(1);
  });
});
