import { describe, expect, it } from 'vitest';
import { toCoreConnectionOptions } from '../../src/connection-option-aliases.js';
import { isRunningNewDriverWithBD, snowflake } from './utils/index.js';

const FAKE_CONNECTION_OPTIONS = {
  account: 'test-account',
  username: 'test-user',
  password: 'test-password',
};

describe('retryTimeout', () => {
  it.each([0, 2, 400, -1])('should accept retryTimeout %s', (retryTimeout) => {
    if (isRunningNewDriverWithBD('BD#91')) {
      expect(toCoreConnectionOptions({ retryTimeout })).toEqual({
        login_timeout: String(retryTimeout),
      });
    }
    snowflake.createConnection({ ...FAKE_CONNECTION_OPTIONS, retryTimeout });
  });

  it('should handle a fractional retryTimeout', () => {
    if (isRunningNewDriverWithBD('BD#91')) {
      expect(() =>
        snowflake.createConnection({ ...FAKE_CONNECTION_OPTIONS, retryTimeout: 1.5 }),
      ).toThrow("Expected type Int for parameter 'login_timeout'");
    } else {
      snowflake.createConnection({ ...FAKE_CONNECTION_OPTIONS, retryTimeout: 1.5 });
    }
  });

  it.each(['invalid' as unknown as number, Number.NaN])(
    'should reject retryTimeout %s',
    (retryTimeout) => {
      if (isRunningNewDriverWithBD('BD#91')) {
        expect(() =>
          snowflake.createConnection({ ...FAKE_CONNECTION_OPTIONS, retryTimeout }),
        ).toThrow("Expected type Int for parameter 'login_timeout'");
      } else {
        expect(() =>
          snowflake.createConnection({ ...FAKE_CONNECTION_OPTIONS, retryTimeout }),
        ).toThrow(expect.objectContaining({ name: 'InvalidParameterError', message: 'undefined' }));
      }
    },
  );
});
