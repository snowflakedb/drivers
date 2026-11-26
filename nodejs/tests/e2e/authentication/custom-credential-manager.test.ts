import { afterAll, beforeAll, beforeEach, describe, expect, it } from 'vitest';
import type { CustomCredentialManager } from '../../../src/index.js';
import type { ConnectionOptions } from '../../types/sdk-types.js';
import { createLiveConnection } from '../utils/fixtures.js';
import {
  IS_RUNNING_FOR_OLD_DRIVER,
  isRunningNewDriverWithBD,
  resetGlobalConfig,
  snowflake,
} from '../utils/index.js';
import {
  jsonResponse,
  loginSuccess,
  logoutSuccess,
  WiremockServer,
} from '../utils/wiremock/index.js';

describe('customCredentialManager', () => {
  let wiremock: WiremockServer;

  beforeAll(async () => {
    wiremock = await WiremockServer.spawn();
  });

  beforeEach(async () => {
    resetGlobalConfig();
    await wiremock.reset();
    await wiremock.stub(logoutSuccess());
  });

  afterAll(async () => {
    resetGlobalConfig();
    await wiremock.destroy();
  });

  function createConnectedMfaConnection(options: Partial<ConnectionOptions> = {}) {
    return createLiveConnection({
      ...wiremock.connectionOptions,
      account: 'test_account',
      username: 'test_user',
      password: 'test_password',
      authenticator: 'USERNAME_PASSWORD_MFA',
      ...(isRunningNewDriverWithBD('BD#117') ? {} : { clientRequestMFAToken: true }),
      ...options,
    });
  }

  it('should delegate token storage to synchronous callbacks', async () => {
    const manager: CustomCredentialManager & { tokens: Map<string, string> } = {
      tokens: new Map(),
      read(key) {
        return this.tokens.get(key) ?? null;
      },
      write(key, token) {
        this.tokens.set(key, token);
      },
      remove(key) {
        this.tokens.delete(key);
      },
    };
    snowflake.configure({ customCredentialManager: manager });
    await wiremock.stub(loginSuccess({ mfaToken: 'stored_mfa_token' }));

    await createConnectedMfaConnection();

    expect([...manager.tokens.entries()]).toEqual([
      [expect.stringMatching(/^SnowflakeTokenCache\.v2\.MfaToken\./), 'stored_mfa_token'],
    ]);
  });

  it('should not call callbacks when token caching is disabled', async () => {
    const calls: string[] = [];
    snowflake.configure({
      customCredentialManager: {
        read: () => {
          calls.push('read');
          return null;
        },
        write: () => {
          calls.push('write');
        },
        remove: () => {
          calls.push('remove');
        },
      },
    });
    await wiremock.stub(loginSuccess({ mfaToken: 'unstored_mfa_token' }));

    await createConnectedMfaConnection(
      isRunningNewDriverWithBD('BD#117')
        ? { clientStoreTemporaryCredential: false }
        : { clientRequestMFAToken: false },
    );

    expect(calls).toEqual([]);
  });

  it('should replace existing callbacks and await asynchronous reads', async () => {
    let supersededReadCalled = false;
    snowflake.configure({
      customCredentialManager: {
        read: () => {
          supersededReadCalled = true;
          return 'superseded_mfa_token';
        },
        write: () => undefined,
        remove: () => undefined,
      },
    });
    let readKey: string | undefined;
    const manager: CustomCredentialManager = {
      read: async (key) => {
        readKey = key;
        return 'cached_mfa_token';
      },
      write: async () => undefined,
      remove: async () => undefined,
    };
    snowflake.configure({ customCredentialManager: manager });
    await wiremock.stub(loginSuccess());

    await createConnectedMfaConnection();

    expect(readKey).toMatch(/^SnowflakeTokenCache\.v2\.MfaToken\./);
    expect(supersededReadCalled).toBe(false);
    const [loginRequest] = await wiremock.findRequests('/session/v1/login-request.*');
    expect(JSON.parse(loginRequest.body).data).toEqual(
      expect.objectContaining({
        TOKEN: 'cached_mfa_token',
      }),
    );
  });

  it('should treat a rejected cache read as a miss without crashing Node', async () => {
    let writtenToken: string | undefined;
    snowflake.configure({
      customCredentialManager: {
        read: async () => {
          throw new Error('cache read failed');
        },
        write: async (_key, token) => {
          writtenToken = token;
        },
        remove: async () => undefined,
      },
    });
    await wiremock.stub(loginSuccess({ mfaToken: 'new_mfa_token' }));

    if (isRunningNewDriverWithBD('BD#92')) {
      await createConnectedMfaConnection();
      const [loginRequest] = await wiremock.findRequests('/session/v1/login-request.*');
      expect(JSON.parse(loginRequest.body).data).not.toHaveProperty('TOKEN');
      expect(writtenToken).toBe('new_mfa_token');
    } else {
      await expect(createConnectedMfaConnection()).rejects.toThrow('cache read failed');
      expect(await wiremock.findRequests('/session/v1/login-request.*')).toEqual([]);
    }
  });

  it('should connect when a cache write throws', async () => {
    snowflake.configure({
      customCredentialManager: {
        read: () => null,
        write: () => {
          throw new Error('cache write failed');
        },
        remove: () => undefined,
      },
    });
    await wiremock.stub(loginSuccess({ mfaToken: 'new_mfa_token' }));

    if (isRunningNewDriverWithBD('BD#92')) {
      const connection = await createConnectedMfaConnection();
      expect(connection.isUp()).toBe(true);
    } else {
      const connecting = createConnectedMfaConnection();
      connecting.catch(() => undefined);
      // The old driver never settles connect() here; the timeout is only the observation window.
      const outcome = await Promise.race([
        connecting.then(() => 'connected'),
        new Promise((resolve) => setTimeout(() => resolve('pending'), 5000)),
      ]);
      expect(outcome).toBe('pending');
    }
  });

  it.skipIf(IS_RUNNING_FOR_OLD_DRIVER)(
    'should remove a rejected cached token before retrying login',
    async () => {
      let firstRead = true;
      let removedKey: string | undefined;
      snowflake.configure({
        customCredentialManager: {
          read: () => {
            if (firstRead) {
              firstRead = false;
              return 'stale_mfa_token';
            }
            return null;
          },
          write: () => undefined,
          remove: (key) => {
            removedKey = key;
          },
        },
      });
      await wiremock.stub({
        scenarioName: 'cached token retry',
        requiredScenarioState: 'Started',
        newScenarioState: 'cached token rejected',
        request: {
          method: 'POST',
          urlPathPattern: '/session/v1/login-request.*',
        },
        response: jsonResponse(200, {
          success: false,
          code: '390120',
          message: 'cached token rejected',
        }),
      });
      await wiremock.stub({
        ...loginSuccess(),
        scenarioName: 'cached token retry',
        requiredScenarioState: 'cached token rejected',
      });

      await createConnectedMfaConnection();

      expect(removedKey).toMatch(/^SnowflakeTokenCache\.v2\.MfaToken\./);
      const loginRequests = await wiremock.findRequests('/session/v1/login-request.*');
      expect(loginRequests).toHaveLength(2);
      expect(JSON.parse(loginRequests[1].body).data).not.toHaveProperty('TOKEN');
    },
  );
});
