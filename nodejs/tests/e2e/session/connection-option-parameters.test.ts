import { describe, expect, it } from 'vitest';
import { createConnection, createLiveConnection } from '../utils/fixtures.js';
import { baseConnectionOptions, isRunningNewDriverWithBD, snowflake } from '../utils/index.js';

describe('account parameter', () => {
  it('should reject a missing account', async () => {
    if (isRunningNewDriverWithBD('BD#87')) {
      const connection = createConnection({ account: undefined, host: undefined });
      await expect(connection.connectAsync()).rejects.toMatchObject({
        name: 'Error',
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

  it('should reject an empty account', async () => {
    if (isRunningNewDriverWithBD('BD#87')) {
      const connection = createConnection({ account: '' });
      await expect(connection.connectAsync()).rejects.toMatchObject({
        name: 'Error',
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

  it('should reject an invalid account', async () => {
    if (isRunningNewDriverWithBD('BD#87')) {
      const connection = createConnection({ account: 'acct/x' });
      await expect(connection.connectAsync()).rejects.toMatchObject({
        name: 'Error',
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

  it('should not fill host or account from accessUrl alone', async () => {
    const options = {
      accessUrl: 'https://access-url.snowflakecomputing.com',
      username: 'username',
      password: 'password',
    };
    if (isRunningNewDriverWithBD('BD#88')) {
      const connection = snowflake.createConnection(options);
      await expect(connection.connectAsync()).rejects.toMatchObject({
        name: 'Error',
        code: undefined,
        message: expect.stringContaining("Missing required parameter 'account'"),
      });
    } else {
      expect(() => snowflake.createConnection(options)).not.toThrow();
    }
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
