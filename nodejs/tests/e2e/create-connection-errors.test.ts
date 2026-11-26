import { describe, expect, it, onTestFinished, vi } from 'vitest';
import { isRunningNewDriverWithBD, resetGlobalConfig, snowflake } from './utils/index.js';

const FAKE_CONNECTION_OPTIONS = {
  account: 'test-account',
  username: 'test-user',
  password: 'test-password',
};

describe('createConnection', () => {
  it('should report a non-numeric jwtTimeout under its Node.js option name', () => {
    const create = () =>
      snowflake.createConnection({
        ...FAKE_CONNECTION_OPTIONS,
        jwtTimeout: 'abc' as unknown as number,
      });
    if (isRunningNewDriverWithBD('BD#154')) {
      expect(create).toThrow(
        expect.objectContaining({
          name: 'Error',
          code: undefined,
          message:
            "Invalid argument: [Error] jwtTimeout: Expected type Int for parameter 'jwtTimeout', got String",
        }),
      );
    } else {
      create();
    }
  });

  it('should report a non-numeric timeout under its Node.js option name', () => {
    const create = () =>
      snowflake.createConnection({
        ...FAKE_CONNECTION_OPTIONS,
        timeout: 'abc' as unknown as number,
      });
    if (isRunningNewDriverWithBD('BD#122')) {
      expect(create).toThrow(
        expect.objectContaining({
          name: 'Error',
          code: undefined,
          message:
            "Invalid argument: [Error] timeout: Expected type Int for parameter 'timeout', got String",
        }),
      );
    } else {
      create();
    }
  });

  it('should warn about an unknown option', async () => {
    const warnings: string[] = [];
    if (isRunningNewDriverWithBD('BD#102')) {
      snowflake.configure({
        customLogger: {
          error: () => {},
          warn: (message: string) => warnings.push(message),
          info: () => {},
          debug: () => {},
          trace: () => {},
        },
      });
      onTestFinished(() => resetGlobalConfig());
    }

    snowflake.createConnection({
      ...FAKE_CONNECTION_OPTIONS,
      // @ts-expect-error plain JavaScript callers can pass any key
      notARealOption: 'x',
    });

    if (isRunningNewDriverWithBD('BD#102')) {
      await vi.waitUntil(() =>
        warnings.some((w) => w.includes("Unknown parameter 'notARealOption'")),
      );
    }
  });
});
