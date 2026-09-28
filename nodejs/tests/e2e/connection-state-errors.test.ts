import { afterAll, beforeAll, beforeEach, describe, expect, it, onTestFinished } from 'vitest';
import { createConnection, createLiveConnection } from './utils/fixtures.js';
import { connectAsyncWithErrorBD, destroyConnectionAsync, executeAsync } from './utils/index.js';
import {
  delayedLoginFailure,
  delayedLoginSuccess,
  logoutSuccess,
  WiremockServer,
} from './utils/wiremock/index.js';

describe('Connection State Errors', () => {
  it('should refuse to connect an established connection again', async () => {
    const connection = await createLiveConnection();

    await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject({
      name: 'ClientError',
      code: 405502,
      sqlState: '08002',
      message: 'Already connected.',
    });
  });

  it('should refuse to connect a destroyed connection again', async () => {
    const connection = await createLiveConnection({}, false);
    await destroyConnectionAsync(connection);

    await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject({
      name: 'ClientError',
      code: 405503,
      sqlState: '08003',
      message: 'Connection already terminated. Cannot connect again.',
    });
  });

  it('should refuse to connect a connection whose login failed', async () => {
    const connection = createConnection({
      username: 'no_such_user_for_e2e',
    });
    await expect(connectAsyncWithErrorBD(connection)).rejects.toThrow();

    await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject({
      name: 'ClientError',
      code: 405503,
      sqlState: '08003',
      message: 'Connection already terminated. Cannot connect again.',
    });
  });

  it('rejects a statement issued before the connection is established', async () => {
    const connection = createConnection();
    await expect(executeAsync(connection, 'select 1')).rejects.toMatchObject({
      error: {
        name: 'ClientError',
        code: 407001,
        sqlState: '08003',
        message: 'Unable to perform operation because a connection was never established.',
      },
    });
  });

  it('rejects a statement issued after the connection is destroyed', async () => {
    const connection = await createLiveConnection({}, false);
    await destroyConnectionAsync(connection);
    await expect(executeAsync(connection, 'select 1')).rejects.toMatchObject({
      error: {
        name: 'ClientError',
        code: 407002,
        sqlState: '08003',
        message: 'Unable to perform operation using terminated connection.',
        isFatal: true,
      },
    });
  });

  it('rejects a statement issued after the login failed', async () => {
    const connection = createConnection({
      username: 'no_such_user_for_e2e',
    });
    await expect(connectAsyncWithErrorBD(connection)).rejects.toThrow();
    await expect(executeAsync(connection, 'select 1')).rejects.toMatchObject({
      error: {
        name: 'ClientError',
        code: 407002,
        sqlState: '08003',
        message: 'Unable to perform operation using terminated connection.',
        isFatal: true,
      },
    });
  });

  it('should refuse to destroy a connection that was never established', async () => {
    const connection = createConnection();
    await expect(destroyConnectionAsync(connection)).rejects.toMatchObject({
      name: 'ClientError',
      code: 406501,
      message: 'Not connected, so nothing to destroy.',
    });
  });

  it('should refuse to destroy a connection that is already destroyed', async () => {
    const connection = await createLiveConnection({}, false);
    await destroyConnectionAsync(connection);
    await expect(destroyConnectionAsync(connection)).rejects.toMatchObject({
      name: 'ClientError',
      code: 406502,
      message: 'Already disconnected.',
    });
  });

  it('should refuse to destroy a connection whose login failed', async () => {
    const connection = createConnection({
      username: 'no_such_user_for_e2e',
    });
    await expect(connectAsyncWithErrorBD(connection)).rejects.toThrow();
    await expect(destroyConnectionAsync(connection)).rejects.toMatchObject({
      name: 'ClientError',
      code: 406502,
      message: 'Already disconnected.',
    });
  });
});

const LOGIN_IN_FLIGHT_DELAY_MS = 1000;
const LOGIN_REQUEST_PATH = '/session/v1/login-request.*';

describe('Connection State Errors with a delayed login', () => {
  let wiremock: WiremockServer;

  beforeAll(async () => {
    wiremock = await WiremockServer.spawn();
  });

  afterAll(async () => {
    await wiremock.destroy();
  });

  describe('that succeeds', () => {
    beforeEach(async () => {
      await wiremock.reset();
      await wiremock.stub(delayedLoginSuccess(LOGIN_IN_FLIGHT_DELAY_MS));
      await wiremock.stub(logoutSuccess());
    });

    it('should refuse a second connect while login is in progress', async () => {
      const connection = createConnection(wiremock.connectionOptions);
      onTestFinished(async () => {
        if (connection.isUp()) {
          await destroyConnectionAsync(connection);
        }
      });

      const login = connectAsyncWithErrorBD(connection);
      await expect
        .poll(async () => (await wiremock.findRequests(LOGIN_REQUEST_PATH)).length, {
          interval: 50,
          timeout: 2500,
        })
        .toBeGreaterThan(0);

      await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject({
        name: 'ClientError',
        code: 405501,
        sqlState: '08002',
        message: 'Connection already in progress.',
      });
      await expect(login).resolves.toBeUndefined();
    });

    it('should destroy a connection once a login in flight has settled', async () => {
      const connection = createConnection(wiremock.connectionOptions);
      onTestFinished(async () => {
        if (connection.isUp()) {
          await destroyConnectionAsync(connection);
        }
      });

      const login = connectAsyncWithErrorBD(connection);
      await expect
        .poll(async () => (await wiremock.findRequests(LOGIN_REQUEST_PATH)).length, {
          interval: 50,
          timeout: 2500,
        })
        .toBeGreaterThan(0);
      const destroy = destroyConnectionAsync(connection);

      await expect(login).resolves.toBeUndefined();
      await expect(destroy).resolves.toBeUndefined();
    });
  });

  describe('that fails', () => {
    beforeEach(async () => {
      await wiremock.reset();
      await wiremock.stub(delayedLoginFailure(LOGIN_IN_FLIGHT_DELAY_MS));
    });

    it('should refuse to destroy a connection whose login in flight failed', async () => {
      const connection = createConnection(wiremock.connectionOptions);
      onTestFinished(async () => {
        if (connection.isUp()) {
          await destroyConnectionAsync(connection);
        }
      });

      const login = connectAsyncWithErrorBD(connection);
      await expect
        .poll(async () => (await wiremock.findRequests(LOGIN_REQUEST_PATH)).length, {
          interval: 50,
          timeout: 2500,
        })
        .toBeGreaterThan(0);
      const destroy = destroyConnectionAsync(connection);

      await expect(login).rejects.toThrow();
      await expect(destroy).rejects.toMatchObject({
        name: 'ClientError',
        code: 406502,
        message: 'Already disconnected.',
      });
    });
  });
});
