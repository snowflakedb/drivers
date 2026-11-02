import { afterAll, beforeAll, beforeEach, describe, it } from 'vitest';
import { createLiveConnection } from './utils/fixtures.js';
import {
  baseConnectionOptions,
  executeAsync,
  resetGlobalConfig,
  snowflake,
} from './utils/index.js';
import {
  destroyConnectionAndCollectTelemetryLogs,
  expectInBandTelemetryLog,
} from './utils/telemetry.js';
import { proxyAllTo, telemetrySuccess, WiremockServer } from './utils/wiremock/index.js';

const PRECISION_LOSS_EVENT = 'selecting_with_precision_loss';

describe('In-band telemetry', () => {
  let wiremock: WiremockServer;

  beforeAll(async () => {
    snowflake.configure({
      customLogger: { error() {}, warn() {}, info() {}, debug() {}, trace() {} },
    });
    wiremock = await WiremockServer.spawn();
    await wiremock.stub(proxyAllTo(`https://${baseConnectionOptions.host}`));
    await wiremock.stub(telemetrySuccess());
  });

  afterAll(async () => {
    await wiremock.destroy();
    resetGlobalConfig();
  });

  beforeEach(async () => {
    await wiremock.clearRequests();
  });

  it('should report the application option as the telemetry source', async () => {
    const connection = await createLiveConnection({
      ...wiremock.connectionOptions,
      application: 'MY_APPLICATION',
    });
    const { statement } = await executeAsync(connection, 'SELECT 9007199254740995::INT');

    const logs = await destroyConnectionAndCollectTelemetryLogs(
      wiremock,
      connection,
      PRECISION_LOSS_EVENT,
      1,
    );
    expectInBandTelemetryLog(
      logs[0],
      PRECISION_LOSS_EVENT,
      statement.getQueryId(),
      'MY_APPLICATION',
    );
  });
});
