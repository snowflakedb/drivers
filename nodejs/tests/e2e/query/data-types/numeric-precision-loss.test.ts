import { afterAll, beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { createLiveConnection } from '../../utils/fixtures.js';
import {
  baseConnectionOptions,
  executeAsync,
  isRunningNewDriverWithBD,
  NOT_IMPLEMENTED_IN_NEW_DRIVER,
  resetGlobalConfig,
  sleepAsync,
  snowflake,
} from '../../utils/index.js';
import { proxyAllTo, telemetrySuccess, WiremockServer } from '../../utils/wiremock/index.js';
import { expectBigIntValue } from '../utils.js';

describe('Numeric precision loss', () => {
  let warnsLogged: string[] = [];
  let wiremock: WiremockServer;

  beforeAll(async () => {
    snowflake.configure({
      customLogger: {
        error() {},
        warn(message: string) {
          warnsLogged.push(message);
        },
        info() {},
        debug() {},
        trace() {},
      },
    });
    // The precision-loss telemetry POST /telemetry/send is fire-and-forget, so a live e2e run
    // cannot observe it. WireMock reverse-proxies everything to the real account while a
    // higher-priority stub captures the telemetry POST for inspection.
    wiremock = await WiremockServer.spawn();
    await wiremock.stub(proxyAllTo(`https://${baseConnectionOptions.host}`));
    await wiremock.stub(telemetrySuccess());
  });

  afterAll(async () => {
    await wiremock.destroy();
    resetGlobalConfig();
  });

  beforeEach(async () => {
    warnsLogged = [];
    await wiremock.clearRequests();
  });

  function collectPrecisionLossWarns(): string[] {
    return warnsLogged.filter((message) =>
      message.includes('Query result precision loss detected when converting'),
    );
  }

  async function collectPrecisionLossTelemetryEvents(): Promise<string[]> {
    await sleepAsync(500);
    const requests = await wiremock.findRequests('/telemetry/send.*');
    return requests
      .flatMap((request) => {
        const payload = JSON.parse(request.body) as {
          logs: { message: { type: string; value: { queryId: string } } }[];
        };
        return payload.logs.map((log) => log.message);
      })
      .filter((message) => message.type === 'selecting_with_precision_loss')
      .map((message) => message.value.queryId);
  }

  it('should return rounded Numbers for fixed types, warn for unsafe values, and send precision loss telemetry only once per query', async () => {
    const connection = await createLiveConnection(wiremock.connectionOptions);
    const { rows, statement } = await executeAsync(
      connection,
      // Multiple rows to ensure telemetry is fired per query rather than row
      `SELECT 9007199254740995::INT,
              42::INT,
              NULL::INT,
              123456789012345678901234567890.12::NUMBER(38,2),
              1.25::NUMBER(10,2),
              NULL::NUMBER(38,2)
         UNION ALL
       SELECT -9007199254740995::INT,
              42::INT,
              NULL::INT,
              -123456789012345678901234567890.12::NUMBER(38,2),
              1.25::NUMBER(10,2),
              NULL::NUMBER(38,2)`,
    );
    expect(rows.map((row) => Object.values(row))).toEqual([
      [9007199254740996, 42, null, 1.2345678901234568e29, 1.25, null],
      [-9007199254740996, 42, null, -1.2345678901234568e29, 1.25, null],
    ]);
    if (!NOT_IMPLEMENTED_IN_NEW_DRIVER) {
      expect(collectPrecisionLossWarns()).toEqual([
        expect.stringContaining('9007199254740995'),
        expect.stringContaining('123456789012345678901234567890.12'),
        expect.stringContaining('-9007199254740995'),
        expect.stringContaining('-123456789012345678901234567890.12'),
      ]);
      expect(await collectPrecisionLossTelemetryEvents()).toEqual([statement.getQueryId()]);
    }
  });

  it('should return rounded Numbers for float without warning or telemetry (precision loss is a known server-side behavior)', async () => {
    const connection = await createLiveConnection(wiremock.connectionOptions);
    const { rows } = await executeAsync(
      connection,
      `SELECT 9007199254740930.13231312::FLOAT,
              -9007199254740930.13231312::FLOAT,
              1.5::FLOAT,
              NULL::FLOAT`,
    );
    expect(Object.values(rows[0])).toEqual([9007199254740930, -9007199254740930, 1.5, null]);
    expect(collectPrecisionLossWarns()).toHaveLength(0);
    expect(await collectPrecisionLossTelemetryEvents()).toHaveLength(0);
  });

  it('should keep exact int digits as BigInt under jsTreatIntegerAsBigInt and still lose precision for fixed and float', async () => {
    const connection = await createLiveConnection({
      ...wiremock.connectionOptions,
      jsTreatIntegerAsBigInt: true,
    });
    const { rows, statement } = await executeAsync(
      connection,
      `SELECT 9007199254740995::INT,
              123456789012345678901234567890.12::NUMBER(38,2),
              9007199254740930.13231312::FLOAT`,
    );

    const [intValue, fixedValue, floatValue] = Object.values(rows[0]);
    expectBigIntValue(intValue, '9007199254740995');
    expect(fixedValue).toBe(1.2345678901234568e29);
    expect(floatValue).toBe(9007199254740930);
    if (!NOT_IMPLEMENTED_IN_NEW_DRIVER) {
      expect(collectPrecisionLossWarns()).toEqual([
        expect.stringContaining('123456789012345678901234567890.12'),
      ]);
      expect(await collectPrecisionLossTelemetryEvents()).toEqual([statement.getQueryId()]);
    }
  });

  it('should keep exact fixed type digit strings under fetchAsString', async () => {
    const connection = await createLiveConnection(wiremock.connectionOptions);
    const { rows, statement } = await executeAsync(
      connection,
      `SELECT 9007199254740995::INT, 123456789012345678901234567890.12::NUMBER(38,2),
              9007199254740930.13231312::FLOAT`,
      { fetchAsString: ['Number'] },
    );

    const [intValue, fixedValue, floatValue] = Object.values(rows[0]);
    expect(intValue).toBe('9007199254740995');
    expect(fixedValue).toBe('123456789012345678901234567890.12');
    expect(typeof floatValue).toBe('string');
    if (isRunningNewDriverWithBD('BD#40')) {
      expect(collectPrecisionLossWarns()).toHaveLength(0);
      expect(await collectPrecisionLossTelemetryEvents()).toHaveLength(0);
    } else {
      expect(collectPrecisionLossWarns()).toEqual([
        expect.stringContaining('9007199254740995'),
        expect.stringContaining('123456789012345678901234567890.12'),
      ]);
      expect(await collectPrecisionLossTelemetryEvents()).toEqual([statement.getQueryId()]);
    }
  });
});
