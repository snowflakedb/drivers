import { describe, expect, it } from 'vitest';
import { isRunningNewDriverWithBD, snowflake } from './utils/index.js';

const FAKE_CONNECTION_OPTIONS = {
  account: 'test-account',
  username: 'test-user',
  password: 'test-password',
};

describe('retryTimeout', () => {
  it.each([0, 2, 400, -1])('should accept retryTimeout %s', (retryTimeout) => {
    snowflake.createConnection({ ...FAKE_CONNECTION_OPTIONS, retryTimeout });
  });

  it('should handle a fractional retryTimeout', () => {
    if (isRunningNewDriverWithBD('BD#91')) {
      expect(() =>
        snowflake.createConnection({ ...FAKE_CONNECTION_OPTIONS, retryTimeout: 1.5 }),
      ).toThrow(
        expect.objectContaining({
          name: 'Error',
          code: undefined,
          message:
            "Invalid argument: [Error] retryTimeout: Expected type Int for parameter 'retryTimeout', got String",
        }),
      );
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
        ).toThrow(
          expect.objectContaining({
            name: 'Error',
            code: undefined,
            message:
              "Invalid argument: [Error] retryTimeout: Expected type Int for parameter 'retryTimeout', got String",
          }),
        );
      } else {
        expect(() =>
          snowflake.createConnection({ ...FAKE_CONNECTION_OPTIONS, retryTimeout }),
        ).toThrow(expect.objectContaining({ name: 'InvalidParameterError', message: 'undefined' }));
      }
    },
  );
});
