import { describe, expect, it } from 'vitest';
import { isRunningNewDriverWithBD, snowflake } from './utils/index.js';

const FAKE_CONNECTION_OPTIONS = {
  account: 'test-account',
  username: 'test-user',
  password: 'test-password',
};

describe('browserActionTimeout', () => {
  it.each([120000, 1500, 999, 1700, 12500, 314200])(
    'should accept browserActionTimeout %s',
    (browserActionTimeout) => {
      snowflake.createConnection({ ...FAKE_CONNECTION_OPTIONS, browserActionTimeout });
    },
  );

  it.each([0, -1])('should handle browserActionTimeout %s', (browserActionTimeout) => {
    if (isRunningNewDriverWithBD('BD#59')) {
      snowflake.createConnection({ ...FAKE_CONNECTION_OPTIONS, browserActionTimeout });
    } else {
      expect(() =>
        snowflake.createConnection({ ...FAKE_CONNECTION_OPTIONS, browserActionTimeout }),
      ).toThrow(
        expect.objectContaining({
          name: 'InvalidParameterError',
          code: 404040,
          message: 'Invalid browser timeout value. The specified value must be a positive number.',
        }),
      );
    }
  });

  it('should handle a fractional browserActionTimeout', () => {
    if (isRunningNewDriverWithBD('BD#59')) {
      snowflake.createConnection({ ...FAKE_CONNECTION_OPTIONS, browserActionTimeout: 1.5 });
    } else {
      expect(() =>
        snowflake.createConnection({ ...FAKE_CONNECTION_OPTIONS, browserActionTimeout: 1.5 }),
      ).toThrow(
        expect.objectContaining({
          name: 'InvalidParameterError',
          code: 404040,
          message: 'Invalid browser timeout value. The specified value must be a positive number.',
        }),
      );
    }
  });

  it.each(['invalid' as unknown as number, Number.NaN])(
    'should reject browserActionTimeout %s',
    (browserActionTimeout) => {
      if (isRunningNewDriverWithBD('BD#59')) {
        expect(() =>
          snowflake.createConnection({ ...FAKE_CONNECTION_OPTIONS, browserActionTimeout }),
        ).toThrow(
          expect.objectContaining({
            name: 'Error',
            code: undefined,
            message:
              "Invalid argument: [Error] browserActionTimeout: Expected type Int for parameter 'browserActionTimeout', got String",
          }),
        );
      } else {
        expect(() =>
          snowflake.createConnection({ ...FAKE_CONNECTION_OPTIONS, browserActionTimeout }),
        ).toThrow(
          expect.objectContaining({
            name: 'InvalidParameterError',
            code: 404040,
            message:
              'Invalid browser timeout value. The specified value must be a positive number.',
          }),
        );
      }
    },
  );
});
