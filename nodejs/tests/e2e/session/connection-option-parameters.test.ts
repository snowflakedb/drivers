import { afterAll, beforeAll, beforeEach, describe, expect, it, onTestFinished } from 'vitest';
import { createConnection, createLiveConnection } from '../utils/fixtures.js';
import {
  baseConnectionOptions,
  destroyConnectionAsync,
  executeAsync,
  isRunningNewDriverWithBD,
  snowflake,
  TEST_CONNECTION_OPTIONS,
} from '../utils/index.js';
import { loginSuccess, logoutSuccess, WiremockServer } from '../utils/wiremock/index.js';

describe('account parameter', () => {
  it('should reject a missing account', async () => {
    if (isRunningNewDriverWithBD('BD#101')) {
      const connection = createConnection({ account: undefined, host: undefined });
      await expect(connection.connectAsync()).rejects.toMatchObject({
        name: 'InvalidParameterError',
        code: undefined,
        message: expect.stringContaining("Missing required parameter 'account'"),
      });
    } else {
      expect(() => createConnection({ account: undefined, host: undefined })).toThrow(
        expect.objectContaining({
          name: 'MissingParameterError',
          code: 404007,
          message: 'An account must be specified.',
        }),
      );
    }
  });

  it('should reject a null account', async () => {
    const options: Record<string, unknown> = {
      ...TEST_CONNECTION_OPTIONS,
      account: null,
      host: undefined,
    };
    if (isRunningNewDriverWithBD('BD#101')) {
      const connection = snowflake.createConnection(options);
      await expect(connection.connectAsync()).rejects.toMatchObject({
        name: 'InvalidParameterError',
        code: undefined,
        message: expect.stringContaining("Missing required parameter 'account'"),
      });
    } else {
      expect(() => snowflake.createConnection(options)).toThrow(
        expect.objectContaining({
          name: 'MissingParameterError',
          code: 404007,
          message: 'An account must be specified.',
        }),
      );
    }
  });

  it('should reject an empty account', async () => {
    if (isRunningNewDriverWithBD('BD#101')) {
      const connection = createConnection({ account: '' });
      await expect(connection.connectAsync()).rejects.toMatchObject({
        name: 'InvalidParameterError',
        code: undefined,
        message: expect.stringContaining("Missing required parameter 'account'"),
      });
    } else {
      expect(() => createConnection({ account: '' })).toThrow(
        expect.objectContaining({
          name: 'InvalidParameterError',
          code: 404045,
          message: 'Invalid account. The specified value must be a valid subdomain string.',
        }),
      );
    }
  });

  it('should connect with host when account is not set', async () => {
    await expect(createLiveConnection({ account: undefined })).resolves.toBeDefined();
  });

  it('should reject an invalid account', async () => {
    if (isRunningNewDriverWithBD('BD#101')) {
      const connection = createConnection({ account: 'acct/x' });
      await expect(connection.connectAsync()).rejects.toMatchObject({
        name: 'InvalidParameterError',
        code: undefined,
        message: expect.stringContaining('Invalid character'),
      });
    } else {
      expect(() => createConnection({ account: 'acct/x' })).toThrow(
        expect.objectContaining({
          name: 'InvalidParameterError',
          code: 404045,
          message: 'Invalid account. The specified value must be a valid subdomain string.',
        }),
      );
    }
  });

  describe('account format', () => {
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

    it.each([
      {
        account: '-acct-',
        accountName: '-acct-',
        code: 404045,
        message: 'Invalid account. The specified value must be a valid subdomain string.',
      },
      {
        account: 'a..b',
        accountName: 'a..b',
        code: 404045,
        message: 'Invalid account. The specified value must be a valid subdomain string.',
      },
      {
        account: 123,
        accountName: '123',
        code: 404008,
        message: 'Invalid account. The specified value must be a string.',
      },
    ])(
      'should log in with account $account that the old driver rejects',
      async ({ account, accountName, code, message }) => {
        const options = {
          ...wiremock.connectionOptions,
          account: account as string,
          username: 'username',
          password: 'password',
        };
        if (isRunningNewDriverWithBD('BD#87')) {
          const connection = snowflake.createConnection(options);
          await connection.connectAsync();
          onTestFinished(async () => {
            await destroyConnectionAsync(connection);
          });

          const [loginRequest] = await wiremock.findRequests('/session/v1/login-request.*');
          const { data } = JSON.parse(loginRequest.body) as { data: { ACCOUNT_NAME: string } };
          expect(data.ACCOUNT_NAME).toBe(accountName);
        } else {
          expect(() => snowflake.createConnection(options)).toThrow(
            expect.objectContaining({ name: 'InvalidParameterError', code, message }),
          );
        }
      },
    );
  });
});

describe('accessUrl parameter', () => {
  it('should connect to accessUrl and ignore host, port and protocol', async () => {
    await expect(
      createLiveConnection({
        accessUrl: `https://${baseConnectionOptions.host}`,
        host: 'ignored-host.snowflakecomputing.com',
        port: 1,
        protocol: 'http',
      }),
    ).resolves.toBeDefined();
  });

  it('should connect with accessUrl when account and host are not set', async () => {
    await expect(
      createLiveConnection({
        accessUrl: `https://${baseConnectionOptions.host}`,
        account: undefined,
        host: undefined,
      }),
    ).resolves.toBeDefined();
  });

  it('should reject an accessUrl that is not a URL', async () => {
    const options = {
      accessUrl: 'not-a-url',
      account: 'account',
      username: 'username',
      password: 'password',
    };
    if (isRunningNewDriverWithBD('BD#89')) {
      const connection = snowflake.createConnection(options);
      await expect(connection.connectAsync()).rejects.toMatchObject({
        name: 'NetworkError',
        code: 401001,
        message: 'Network error. Could not reach Snowflake.',
      });
    } else {
      expect(() => snowflake.createConnection(options)).toThrow(
        expect.objectContaining({
          name: 'InvalidParameterError',
          code: 404010,
          message: 'Invalid accessUrl. The specified value must be a string.',
        }),
      );
    }
  });

  it('should reject an empty accessUrl', async () => {
    const options = {
      accessUrl: '',
      account: 'account',
      username: 'username',
      password: 'password',
    };
    if (isRunningNewDriverWithBD('BD#89')) {
      const connection = snowflake.createConnection(options);
      await expect(connection.connectAsync()).rejects.toMatchObject({
        name: 'NetworkError',
        code: 401001,
        message: 'Network error. Could not reach Snowflake.',
      });
    } else {
      expect(() => snowflake.createConnection(options)).toThrow(
        expect.objectContaining({
          name: 'InvalidParameterError',
          code: 404010,
          message: 'Invalid accessUrl. The specified value must be a string.',
        }),
      );
    }
  });
});

it('should ignore an unknown option', async () => {
  const connection = await createLiveConnection({
    // @ts-expect-error test overwrite
    notARealOption: 'value',
  });
  const { rows } = await executeAsync(connection, 'SELECT 1');
  expect(Object.values(rows[0])).toEqual([1]);
});
