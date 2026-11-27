import { afterAll, afterEach, beforeAll, describe, it, expect, onTestFinished } from 'vitest';
import { createConnection } from './utils/fixtures.js';
import {
  connectAsyncWithErrorBD,
  destroyConnectionAsync,
  isRunningNewDriverWithBD,
  snowflake,
} from './utils/index.js';
import { loginRequestFail, WiremockServer, withResponse } from './utils/wiremock/index.js';

describe('Connection', () => {
  it('should connect with .connect() on valid parameters', async () => {
    const connection = createConnection();
    onTestFinished(async () => {
      await destroyConnectionAsync(connection);
    });
    await new Promise<void>((resolve, reject) => {
      const returned = connection.connect((err) => (err ? reject(err) : resolve()));
      expect(returned).toBe(connection);
    });
    expect(connection.isUp()).toBe(true);
  });

  it('should surface a rejected login through the .connect() callback', async () => {
    const connection = createConnection({
      username: 'incorrect-username',
    });
    const error = await new Promise<Error | undefined>((resolve) => {
      connection.connect((err) => resolve(err));
    });
    expect(error).toMatchObject({
      name: 'OperationFailedError',
      code: '390144',
      message: expect.stringContaining('JWT token is invalid'),
    });
  });

  it('should connect with .connectAsync() on valid parameters', async () => {
    const connection = createConnection();
    onTestFinished(async () => {
      await destroyConnectionAsync(connection);
    });
    await expect(connection.connectAsync()).resolves.toBe(connection);
    expect(connection.isUp()).toBe(true);
  });

  it('should surface a rejected login from .connectAsync()', async () => {
    const connection = createConnection({
      username: 'incorrect-username',
    });
    const expectedError = {
      name: 'OperationFailedError',
      code: '390144',
      message: expect.stringContaining('JWT token is invalid'),
    };
    if (isRunningNewDriverWithBD('BD#11')) {
      await expect(connection.connectAsync()).rejects.toMatchObject(expectedError);
    } else {
      const error = await new Promise<Error | undefined>((resolve) => {
        void connection.connectAsync((err) => resolve(err));
      });
      expect(error).toMatchObject(expectedError);
    }
  });

  it('should reject a connection without a user name', async () => {
    const options = { account: 'a', password: 'p' };
    if (isRunningNewDriverWithBD('BD#101')) {
      const connection = snowflake.createConnection(options);
      await expect(connection.connectAsync()).rejects.toMatchObject({
        name: 'InvalidParameterError',
        code: undefined,
        message:
          "Configuration error: Configuration validation failed (1 issue(s)): user: Missing required parameter 'user'",
      });
    } else {
      expect(() => snowflake.createConnection(options)).toThrow(
        expect.objectContaining({
          name: 'MissingParameterError',
          code: 404003,
          message: 'A user name must be specified.',
        }),
      );
    }
  });

  it('destroys the connection and transitions to a disconnected state', async () => {
    const connection = createConnection();
    await connection.connectAsync();
    await destroyConnectionAsync(connection);
    expect(connection.isUp()).toBe(false);
  });

  describe('isValidAsync', () => {
    it('should report a connection as valid after connect', async () => {
      const connection = createConnection();
      try {
        await connection.connectAsync();
        await expect(connection.isValidAsync()).resolves.toBe(true);
      } finally {
        await destroyConnectionAsync(connection);
      }
    });

    it('should report a connection as invalid after destroy', async () => {
      const connection = createConnection();
      await connection.connectAsync();
      await destroyConnectionAsync(connection);
      await expect(connection.isValidAsync()).resolves.toBe(false);
    });
  });

  describe('wiremock', () => {
    let wiremock: WiremockServer;

    beforeAll(async () => {
      wiremock = await WiremockServer.spawn();
    });

    afterEach(async () => {
      await wiremock.reset();
    });

    afterAll(async () => {
      await wiremock.destroy();
    });

    it('should reject login when Snowflake answers with HTTP 400', async () => {
      await wiremock.stub(withResponse(loginRequestFail(), { status: 400 }));
      const connection = createConnection(wiremock.connectionOptions);
      await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject({
        name: 'RequestFailedError',
        code: 401002,
        message: 'Request to Snowflake failed.',
        response: { statusCode: 400, statusText: 'Bad Request' },
        ...(isRunningNewDriverWithBD('BD#46')
          ? {
              cause: {
                message:
                  'Failed to login: Invalid Snowflake response: Snowflake responded with error status: ' +
                  '400 Bad Request, message: Unexpected response, body length 17.',
              },
            }
          : {}),
      });
    });
  });
});
