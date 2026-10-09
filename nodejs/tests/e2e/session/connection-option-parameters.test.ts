import { describe, expect, it } from 'vitest';
import { createConnection } from '../utils/fixtures.js';
import { isRunningNewDriverWithBD } from '../utils/index.js';

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
