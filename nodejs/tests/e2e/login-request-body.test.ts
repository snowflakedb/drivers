import { createRequire } from 'node:module';
import { afterAll, beforeAll, beforeEach, describe, expect, it } from 'vitest';
import type { ConnectionOptions } from '../types/sdk-types.js';
import { createLiveConnection } from './utils/fixtures.js';
import {
  IS_RUNNING_FOR_OLD_DRIVER,
  isRunningNewDriverWithBD,
  NOT_IMPLEMENTED_IN_NEW_DRIVER,
} from './utils/index.js';
import { loginSuccess, logoutSuccess, WiremockServer } from './utils/wiremock/index.js';

describe('Login Request Body', () => {
  let wiremock: WiremockServer;

  async function connectAndGetLoginRequestData(
    overrides: Partial<ConnectionOptions> = {},
  ): Promise<Record<string, any>> {
    await createLiveConnection({ ...wiremock.connectionOptions, ...overrides });
    const [loginRequest] = await wiremock.findRequests('/session/v1/login-request.*');
    return JSON.parse(loginRequest.body).data;
  }

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

  it('should contain CLIENT_APP_ID and CLIENT_APP_VERSION', async () => {
    const data = await connectAndGetLoginRequestData();
    expect(data.CLIENT_APP_ID).toBe('JavaScript');
    expect(data.CLIENT_APP_VERSION).toBe(
      IS_RUNNING_FOR_OLD_DRIVER
        ? (createRequire(import.meta.url)('snowflake-sdk-old/package.json').version as string)
        : (createRequire(import.meta.url)('snowflake-sdk/package.json').version as string),
    );
  });

  describe('CLIENT_ENVIRONMENT', () => {
    it('should contain APPLICATION if passed in connection config', async () => {
      const { CLIENT_ENVIRONMENT } = await connectAndGetLoginRequestData({
        application: 'test-application',
      });
      expect(CLIENT_ENVIRONMENT.APPLICATION).toBe('test-application');
    });

    it.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)('should contain APPLICATION_PATH', async () => {
      const { CLIENT_ENVIRONMENT } = await connectAndGetLoginRequestData();
      expect(CLIENT_ENVIRONMENT.APPLICATION_PATH).toEqual(expect.any(String));
      expect(CLIENT_ENVIRONMENT.APPLICATION_PATH).not.toBe('');
    });

    it('should contain instruction set architecture (ISA)', async () => {
      const { CLIENT_ENVIRONMENT } = await connectAndGetLoginRequestData();
      if (isRunningNewDriverWithBD('BD#81')) {
        const rustArchByNodeArch: Partial<Record<NodeJS.Architecture, string>> = {
          arm64: 'aarch64',
          x64: 'x86_64',
        };
        expect(CLIENT_ENVIRONMENT.ISA).toBe(rustArchByNodeArch[process.arch]);
      } else {
        expect(CLIENT_ENVIRONMENT.ISA).toBe(process.arch);
      }
    });

    // In order for this to work in the new driver, log_manager should be registered by the bridge.
    it.skipIf(process.platform !== 'linux' || NOT_IMPLEMENTED_IN_NEW_DRIVER)(
      'should contain OS_DETAILS on Linux',
      async () => {
        const { CLIENT_ENVIRONMENT } = await connectAndGetLoginRequestData();
        expect(Object.keys(CLIENT_ENVIRONMENT.OS_DETAILS).length).toBeGreaterThanOrEqual(1);
      },
    );

    it.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER || process.platform === 'linux')(
      'should send null OS_DETAILS on non-Linux platforms',
      async () => {
        const { CLIENT_ENVIRONMENT } = await connectAndGetLoginRequestData();
        expect(CLIENT_ENVIRONMENT.OS_DETAILS).toBeNull();
      },
    );

    it.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)(
      'should contain LIBC_FAMILY and LIBC_VERSION on Linux or null on other platforms',
      async () => {
        const { CLIENT_ENVIRONMENT } = await connectAndGetLoginRequestData();
        if (process.platform === 'linux') {
          expect(CLIENT_ENVIRONMENT.LIBC_FAMILY).toMatch(/^(glibc|musl)$/);
          expect(CLIENT_ENVIRONMENT.LIBC_VERSION).toEqual(expect.any(String));
          expect(CLIENT_ENVIRONMENT.LIBC_VERSION).not.toBe('');
        } else {
          expect(CLIENT_ENVIRONMENT.LIBC_FAMILY).toBeNull();
          expect(CLIENT_ENVIRONMENT.LIBC_VERSION).toBeNull();
        }
      },
    );
  });
});
