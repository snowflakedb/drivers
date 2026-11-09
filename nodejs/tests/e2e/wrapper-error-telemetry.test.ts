import { afterAll, beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { createLiveConnection } from './utils/fixtures.js';
import { destroyConnectionAsync, executeAsync, IS_RUNNING_FOR_OLD_DRIVER } from './utils/index.js';
import {
  jsonResponse,
  loginSuccess,
  logoutSuccess,
  queryRequestFail,
  type StubMapping,
  telemetrySuccess,
  WiremockServer,
} from './utils/wiremock/index.js';

interface TelemetryMessage {
  type: string;
  'exception.type'?: string;
  'exception.source'?: string;
}

describe.skipIf(IS_RUNNING_FOR_OLD_DRIVER)('wrapper_error telemetry', () => {
  let wiremock: WiremockServer;

  beforeAll(async () => {
    wiremock = await WiremockServer.spawn();
  });

  afterAll(async () => {
    await wiremock.destroy();
  });

  beforeEach(async () => {
    await wiremock.reset();
    await wiremock.stub([
      loginSuccess({
        parameters: [{ name: 'CLIENT_TELEMETRY_ENABLED', value: true }],
      }),
      logoutSuccess(),
      telemetrySuccess(),
    ]);
  });

  function createVariantQueryStub(value: string): StubMapping {
    return {
      request: {
        method: 'POST',
        urlPathPattern: '/queries/v1/query-request.*',
      },
      response: jsonResponse(200, {
        success: true,
        data: {
          queryId: '01b2c3d4-0000-0000-0000-000000000001',
          rowtype: [{ name: 'V', type: 'variant', nullable: true }],
          rowset: [[value]],
          total: 1,
          returned: 1,
          queryResultFormat: 'json',
          statementTypeId: 4096,
        },
      }),
    };
  }

  async function getExceptionMessages(): Promise<TelemetryMessage[]> {
    const requests = await wiremock.findRequests('/telemetry/send.*');
    return requests
      .flatMap((request) => {
        const payload = JSON.parse(request.body) as {
          logs: { message: TelemetryMessage }[];
        };
        return payload.logs.map((log) => log.message);
      })
      .filter((message) => message.type === 'exception');
  }

  it('should report a failed execute exactly once', async () => {
    await wiremock.stub(queryRequestFail('002003', 'Object does not exist.'));
    const connection = await createLiveConnection(wiremock.connectionOptions);

    await expect(executeAsync(connection, 'SELECT * FROM MISSING')).rejects.toBeDefined();
    await destroyConnectionAsync(connection);

    expect(await getExceptionMessages()).toEqual([
      expect.objectContaining({
        'exception.type': 'OperationFailedError',
        'exception.source': 'server_error',
      }),
    ]);
  });

  it('should report a connection-level failure', async () => {
    const connection = await createLiveConnection(wiremock.connectionOptions);

    await expect(connection.getQueryStatus('not-a-query-id')).rejects.toBeDefined();
    await destroyConnectionAsync(connection);

    expect(await getExceptionMessages()).toEqual([
      expect.objectContaining({
        'exception.type': 'InvalidParameterError',
        'exception.source': 'api_misuse',
      }),
    ]);
  });

  it('should report a row conversion failure', async () => {
    await wiremock.stub(createVariantQueryStub('{not json'));
    const connection = await createLiveConnection(wiremock.connectionOptions);

    await expect(executeAsync(connection, 'SELECT V')).rejects.toBeDefined();
    await destroyConnectionAsync(connection);

    expect(await getExceptionMessages()).toEqual([
      expect.objectContaining({
        'exception.type': 'Error',
        'exception.source': 'data_conversion',
      }),
    ]);
  });

  it('should report a bind that cannot be serialized', async () => {
    const connection = await createLiveConnection(wiremock.connectionOptions);

    await expect(
      executeAsync(connection, 'SELECT ?', {
        binds: [1n as unknown as number],
      }),
    ).rejects.toBeDefined();
    await destroyConnectionAsync(connection);

    expect(await getExceptionMessages()).toEqual([
      expect.objectContaining({
        'exception.type': 'TypeError',
        'exception.source': 'api_misuse',
      }),
    ]);
  });

  it('should not report successful calls', async () => {
    await wiremock.stub(createVariantQueryStub('{"a": 1}'));
    const connection = await createLiveConnection(wiremock.connectionOptions);

    await executeAsync(connection, 'SELECT V');
    await destroyConnectionAsync(connection);

    expect(await getExceptionMessages()).toEqual([]);
  });
});
