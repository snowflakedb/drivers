import { createRequire } from 'node:module';
import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it } from 'vitest';
import type { Connection } from '../types/sdk-types.js';
import {
  createTestConnection,
  destroyConnectionAsync,
  getSnowflakeSDK,
  IS_RUNNING_FOR_OLD_DRIVER,
} from './utils/index.js';
import { loginSuccess, logoutSuccess, WiremockServer } from './utils/wiremock/index.js';

describe('Login Request Body', () => {
  const snowflake = getSnowflakeSDK();
  let wiremock: WiremockServer;
  let connection: Connection;

  beforeAll(async () => {
    wiremock = await WiremockServer.spawn();
  });

  beforeEach(async () => {
    connection = createTestConnection(snowflake, wiremock.connectionOptions);
    await wiremock.reset();
  });

  afterEach(async () => {
    await wiremock.stub(logoutSuccess());
    await destroyConnectionAsync(connection);
  });

  afterAll(async () => {
    await wiremock.destroy();
  });

  it('should contain CLIENT_APP_ID and CLIENT_APP_VERSION', async () => {
    await wiremock.stub(loginSuccess());
    await connection.connectAsync();

    const [loginRequest] = await wiremock.findRequests('/session/v1/login-request.*');
    const { data } = JSON.parse(loginRequest.body);

    expect(data.CLIENT_APP_ID).toBe('JavaScript');
    expect(data.CLIENT_APP_VERSION).toBe(
      IS_RUNNING_FOR_OLD_DRIVER
        ? (createRequire(import.meta.url)('snowflake-sdk-old/package.json').version as string)
        : (createRequire(import.meta.url)('snowflake-sdk/package.json').version as string),
    );
  });
});
