import { afterAll, beforeAll, beforeEach, describe, expect, it } from 'vitest';
import type { Connection, RowStatement } from '../../../types/sdk-types.js';
import { createLiveConnection } from '../../utils/fixtures.js';
import {
  baseConnectionOptions,
  collectStreamedRows,
  executeAsync,
  isRunningNewDriverWithBD,
  resetGlobalConfig,
  snowflake,
} from '../../utils/index.js';
import {
  destroyConnectionAndCollectTelemetryLogs,
  expectInBandTelemetryLog,
} from '../../utils/telemetry.js';
import { proxyAllTo, telemetrySuccess, WiremockServer } from '../../utils/wiremock/index.js';
import { expectBigIntValue } from '../utils.js';

const PRECISION_LOSS_EVENT = 'selecting_with_precision_loss';

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

  function expectPrecisionLossWarns(lostValues: [rawValue: string, columnIndex: number][]): void {
    expect(collectPrecisionLossWarns()).toEqual(
      lostValues.map(([rawValue, columnIndex]) =>
        expect.stringContaining(
          isRunningNewDriverWithBD('BD#145')
            ? `converting a value in column at index ${columnIndex} to a JavaScript Number`
            : `converting ${rawValue} to a JavaScript Number`,
        ),
      ),
    );
  }

  const destroyConnectionAndCollectPrecisionLossLogs = (
    connection: Connection,
    expectedCount: number,
  ) =>
    destroyConnectionAndCollectTelemetryLogs(
      wiremock,
      connection,
      PRECISION_LOSS_EVENT,
      expectedCount,
    );

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
    expectPrecisionLossWarns([
      ['9007199254740995', 0],
      ['123456789012345678901234567890.12', 3],
      ['-9007199254740995', 0],
      ['-123456789012345678901234567890.12', 3],
    ]);
    const logs = await destroyConnectionAndCollectPrecisionLossLogs(connection, 1);
    expectInBandTelemetryLog(logs[0], PRECISION_LOSS_EVENT, statement.getQueryId());
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
    await destroyConnectionAndCollectPrecisionLossLogs(connection, 0);
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
    expectPrecisionLossWarns([['123456789012345678901234567890.12', 1]]);
    const logs = await destroyConnectionAndCollectPrecisionLossLogs(connection, 1);
    expectInBandTelemetryLog(logs[0], PRECISION_LOSS_EVENT, statement.getQueryId());
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
      await destroyConnectionAndCollectPrecisionLossLogs(connection, 0);
    } else {
      expectPrecisionLossWarns([
        ['9007199254740995', 0],
        ['123456789012345678901234567890.12', 1],
      ]);
      const logs = await destroyConnectionAndCollectPrecisionLossLogs(connection, 1);
      expectInBandTelemetryLog(logs[0], PRECISION_LOSS_EVENT, statement.getQueryId());
    }
  });

  it('should send precision loss telemetry once per query when one connection runs several queries', async () => {
    const connection = await createLiveConnection(wiremock.connectionOptions);
    const { statement: first } = await executeAsync(connection, 'SELECT 9007199254740995::INT');
    const { statement: second } = await executeAsync(connection, 'SELECT 9007199254740997::INT');

    const logs = await destroyConnectionAndCollectPrecisionLossLogs(connection, 2);
    expect(logs.map((log) => log.message.value.queryId).sort()).toEqual(
      [first.getQueryId(), second.getQueryId()].sort(),
    );
  });

  it('should send precision loss telemetry when rows are streamed', async () => {
    const connection = await createLiveConnection(wiremock.connectionOptions);
    const { statement } = await executeAsync(
      connection,
      'SELECT 9007199254740995::INT UNION ALL SELECT 9007199254740997::INT',
      { streamResult: true },
    );
    const rows = await collectStreamedRows(statement as RowStatement);

    expect(rows.map((row) => Object.values(row)[0])).toEqual([9007199254740996, 9007199254740996]);
    const logs = await destroyConnectionAndCollectPrecisionLossLogs(connection, 1);
    expectInBandTelemetryLog(logs[0], PRECISION_LOSS_EVENT, statement.getQueryId());
  });
});
