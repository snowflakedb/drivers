import { createRequire } from 'node:module';
import { expect, vi } from 'vitest';
import type { Connection } from '../../types/sdk-types.js';
import type { WiremockServer } from './wiremock/index.js';
import { destroyConnectionAsync, IS_RUNNING_FOR_OLD_DRIVER, sleepAsync } from './index.js';

interface InBandTelemetryLog {
  timestamp: string | number;
  message: {
    driver_type: string;
    driver_version: string;
    source: string;
    type: string;
    value: { queryId: string };
  };
}

/**
 * Destroys `connection`, waits until WireMock has captured `expectedCount` in-band telemetry logs
 * of `eventType` on `/telemetry/send`, and returns them.
 *
 * The old driver posts each event as soon as it happens, without awaiting the request. The new
 * driver queues them in core and `destroy()` awaits the flush of the batch, so the connection must
 * be destroyed before the captured requests are read.
 */
export async function destroyConnectionAndCollectTelemetryLogs(
  wiremock: WiremockServer,
  connection: Connection,
  eventType: string,
  expectedCount: number,
): Promise<InBandTelemetryLog[]> {
  await destroyConnectionAsync(connection);
  const findLogs = async () => {
    const requests = await wiremock.findRequests('/telemetry/send.*');
    return requests.flatMap((request) => {
      const payload = JSON.parse(request.body) as {
        logs: InBandTelemetryLog[];
      };
      return payload.logs.filter((log) => log.message.type === eventType);
    });
  };
  if (expectedCount === 0) {
    // The absence of a request cannot be polled for: this grace period gives an old-driver
    // request that is still in flight time to reach WireMock and fail the assertion.
    await sleepAsync(500);
    const logs = await findLogs();
    expect(logs).toHaveLength(0);
    return logs;
  } else {
    return vi.waitFor(
      async () => {
        const logs = await findLogs();
        expect(logs).toHaveLength(expectedCount);
        return logs;
      },
      { timeout: 1000, interval: 100 },
    );
  }
}

export function expectInBandTelemetryLog(
  log: InBandTelemetryLog,
  eventType: string,
  queryId: string | undefined,
  source = 'JavaScript',
): void {
  const driverPackage = IS_RUNNING_FOR_OLD_DRIVER ? 'snowflake-sdk-old' : 'snowflake-sdk';
  expect(log).toEqual({
    // The timestamp type differs between drivers and is not visible to users: the old driver sends
    // epoch millis as a number, and the new driver's core serializes every log timestamp as a
    // digit string.
    timestamp: IS_RUNNING_FOR_OLD_DRIVER ? expect.any(Number) : expect.stringMatching(/^\d+$/),
    message: {
      driver_type: 'JavaScript',
      driver_version: createRequire(import.meta.url)(`${driverPackage}/package.json`).version,
      source,
      type: eventType,
      value: { queryId },
    },
  });
}
