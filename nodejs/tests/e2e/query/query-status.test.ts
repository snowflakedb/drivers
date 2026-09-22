import { describe, expect, it } from 'vitest';
import type { Connection } from '../../types/sdk-types.js';
import { createConnection } from '../utils/fixtures.js';
import { connectionIsAnErrorWithBD, isRunningNewDriverWithBD } from '../utils/index.js';

describe('Query status', () => {
  const connection: Connection = createConnection();

  describe('isStillRunning', () => {
    it.each([
      { status: 'RUNNING', expected: true },
      { status: 'ABORTING', expected: false },
      { status: 'SUCCESS', expected: false },
      { status: 'FAILED_WITH_ERROR', expected: false },
      { status: 'ABORTED', expected: false },
      { status: 'QUEUED', expected: true },
      { status: 'FAILED_WITH_INCIDENT', expected: false },
      { status: 'DISCONNECTED', expected: false },
      { status: 'RESUMING_WAREHOUSE', expected: true },
      { status: 'QUEUED_REPARING_WAREHOUSE', expected: true },
      { status: 'RESTARTED', expected: false },
      { status: 'BLOCKED', expected: isRunningNewDriverWithBD('BD#41') },
      { status: 'NO_DATA', expected: true },
    ] as const)('should report $status as still running: $expected', ({ status, expected }) => {
      expect(connection.isStillRunning(status)).toBe(expected);
    });
  });

  describe('isAnError', () => {
    it.each([
      { status: 'RUNNING', expected: false },
      { status: 'ABORTING', expected: true },
      { status: 'SUCCESS', expected: false },
      { status: 'FAILED_WITH_ERROR', expected: true },
      { status: 'ABORTED', expected: true },
      { status: 'QUEUED', expected: false },
      { status: 'FAILED_WITH_INCIDENT', expected: true },
      { status: 'DISCONNECTED', expected: true },
      { status: 'RESUMING_WAREHOUSE', expected: false },
      { status: 'QUEUED_REPARING_WAREHOUSE', expected: false },
      { status: 'RESTARTED', expected: false },
      { status: 'BLOCKED', expected: !isRunningNewDriverWithBD('BD#41') },
      { status: 'NO_DATA', expected: false },
    ] as const)('should report $status as an error: $expected', ({ status, expected }) => {
      expect(connectionIsAnErrorWithBD(connection, status)).toBe(expected);
    });
  });
});
