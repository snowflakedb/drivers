import { describe, expect, it } from 'vitest';
import { toCoreConnectionOptions } from '../../src/connection-option-aliases.js';
import { isRunningNewDriverWithBD, snowflake } from './utils/index.js';

const FAKE_CONNECTION_OPTIONS = {
  account: 'test-account',
  username: 'test-user',
  password: 'test-password',
};

describe('browserActionTimeout', () => {
  it('should accept browserActionTimeout 120000', () => {
    if (isRunningNewDriverWithBD('BD#59')) {
      expect(toCoreConnectionOptions({ browserActionTimeout: 120000 })).toEqual({
        browserActionTimeout: '120',
      });
    }
    snowflake.createConnection({ ...FAKE_CONNECTION_OPTIONS, browserActionTimeout: 120000 });
  });

  it.each([1500, 999])('should accept browserActionTimeout %s', (browserActionTimeout) => {
    if (isRunningNewDriverWithBD('BD#59')) {
      expect(toCoreConnectionOptions({ browserActionTimeout })).toEqual({
        browserActionTimeout: '1',
      });
    }
    snowflake.createConnection({ ...FAKE_CONNECTION_OPTIONS, browserActionTimeout });
  });

  it.each([
    [1700, '1'],
    [12500, '12'],
    [314200, '314'],
  ])('should accept browserActionTimeout %s as %s seconds', (browserActionTimeout, seconds) => {
    if (isRunningNewDriverWithBD('BD#59')) {
      expect(toCoreConnectionOptions({ browserActionTimeout })).toEqual({
        browserActionTimeout: seconds,
      });
    }
    snowflake.createConnection({ ...FAKE_CONNECTION_OPTIONS, browserActionTimeout });
  });

  it.each([0, -1])('should handle browserActionTimeout %s', (browserActionTimeout) => {
    if (isRunningNewDriverWithBD('BD#59')) {
      expect(toCoreConnectionOptions({ browserActionTimeout })).toEqual({
        browserActionTimeout: String(browserActionTimeout),
      });
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
      expect(toCoreConnectionOptions({ browserActionTimeout: 1.5 })).toEqual({
        browserActionTimeout: '1',
      });
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
