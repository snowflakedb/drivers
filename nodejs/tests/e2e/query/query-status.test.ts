import { afterAll, beforeAll, beforeEach, describe, expect, it } from 'vitest';
import type { Connection, RowStatement } from '../../types/sdk-types.js';
import { createConnection, createLiveConnection } from '../utils/fixtures.js';
import {
  connectionGetQueryStatusWithBD,
  connectionIsAnErrorWithBD,
  destroyConnectionAsync,
  executeAsync,
  isRunningNewDriverWithBD,
  NOT_IMPLEMENTED_IN_NEW_DRIVER,
} from '../utils/index.js';
import {
  loginSuccess,
  logoutSuccess,
  monitoringQueryFailure,
  monitoringQueryStatus,
  WiremockServer,
} from '../utils/wiremock/index.js';

describe('Query status', () => {
  let connection: Connection;

  beforeAll(async () => {
    connection = await createLiveConnection({}, false);
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  describe('tests/definitions/shared/query/query_status.feature', () => {
    it('should return success status for completed query', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT 1" is executed
      const { statement } = await executeAsync(connection, 'SELECT 1');

      // And Query status is retrieved by query ID
      const status = await connectionGetQueryStatusWithBD(connection, statement.getQueryId()!);

      // Then the query status should indicate success
      expect(status).toBe('SUCCESS');

      // And the query should not be indicated as still running
      expect(connection.isStillRunning(status)).toBe(false);

      // And the query should not be indicated as an error
      expect(connectionIsAnErrorWithBD(connection, status)).toBe(false);
    });

    it('should return error status for failed query', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When An invalid query is executed and the query ID is captured
      const queryId = await executeAsync(
        connection,
        'SELECT * FROM NON_EXISTENT_TABLE_TEST_12345',
      ).then(
        () => {
          throw new Error('expected execute to fail');
        },
        (err: { statement: RowStatement }) => err.statement.getQueryId()!,
      );

      // And Query status is retrieved by query ID
      const status = await connectionGetQueryStatusWithBD(connection, queryId);

      // Then the query status should indicate an error
      expect(connectionIsAnErrorWithBD(connection, status)).toBe(true);

      // And the query should not be indicated as still running
      expect(connection.isStillRunning(status)).toBe(false);
    });

    it.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)(
      'should indicate still running for in-progress query',
      async () => {
        // Given Snowflake client is logged in
        void connection;

        // When A long-running query is submitted asynchronously
        const { statement } = await executeAsync(connection, 'SELECT SYSTEM$WAIT(30)', {
          asyncExec: true,
        });

        // And Query status is retrieved immediately
        const queryId = statement.getQueryId()!;
        const status = await connectionGetQueryStatusWithBD(connection, queryId);

        // Then the query status should indicate still running
        expect(connection.isStillRunning(status)).toBe(true);

        // And the query should not be indicated as an error
        expect(connectionIsAnErrorWithBD(connection, status)).toBe(false);
      },
    );

    it('should return no data status for a non-existent query ID', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query status is retrieved for a non-existent query ID
      const status = await connection.getQueryStatus('00000000-0000-0000-0000-000000000000');

      // Then the query status should indicate no data
      expect(status).toBe(isRunningNewDriverWithBD('BD#43') ? 'NO_DATA' : 'NO_QUERY_DATA');
    });
  });

  describe('isStillRunning', () => {
    const classifierConnection: Connection = createConnection();

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
      expect(classifierConnection.isStillRunning(status)).toBe(expected);
    });
  });

  describe('isAnError', () => {
    const classifierConnection: Connection = createConnection();

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
      expect(connectionIsAnErrorWithBD(classifierConnection, status)).toBe(expected);
    });
  });

  describe('invalid query ID', () => {
    it('should reject getQueryStatus with 460001 for a malformed query id', async () => {
      await expect(connection.getQueryStatus('invalidQueryId')).rejects.toMatchObject({
        name: 'InvalidParameterError',
        code: 460001,
        message: 'Invalid queryId: invalidQueryId',
      });
    });

    it('should reject getQueryStatusThrowIfError with 460001 for a malformed query id', async () => {
      await expect(connection.getQueryStatusThrowIfError('invalidQueryId')).rejects.toMatchObject({
        name: 'InvalidParameterError',
        code: 460001,
        message: 'Invalid queryId: invalidQueryId',
      });
    });
  });

  describe('getQueryStatusThrowIfError', () => {
    it('should throw OperationFailedError for a failed query', async () => {
      const queryId = await executeAsync(
        connection,
        'SELECT * FROM NON_EXISTENT_TABLE_TEST_12345',
      ).then(
        () => {
          throw new Error('expected execute to fail');
        },
        (err: { statement: RowStatement }) => err.statement.getQueryId()!,
      );

      await expect(connection.getQueryStatusThrowIfError(queryId)).rejects.toMatchObject({
        name: 'OperationFailedError',
        code: isRunningNewDriverWithBD('BD#47') ? '002003' : -1,
        message:
          "SQL compilation error:\nObject 'NON_EXISTENT_TABLE_TEST_12345' does not exist or not authorized.",
      });
    });

    it('should return SUCCESS for a completed query', async () => {
      const { statement } = await executeAsync(connection, 'SELECT 1');
      await expect(connection.getQueryStatusThrowIfError(statement.getQueryId()!)).resolves.toBe(
        'SUCCESS',
      );
    });
  });
});

describe.each(['getQueryStatus', 'getQueryStatusThrowIfError'] as const)(
  '%s with Wiremock',
  (method) => {
    let wiremock: WiremockServer;
    const queryId = '12345678-1234-4123-a123-123456789012';

    beforeAll(async () => {
      wiremock = await WiremockServer.spawn();
    });

    beforeEach(async () => {
      await wiremock.reset();
      await wiremock.stub(loginSuccess());
      await wiremock.stub(logoutSuccess());
    });

    afterAll(async () => {
      await wiremock.destroy();
    });

    it('should map an unrecognized server status name', async () => {
      await wiremock.stub(monitoringQueryStatus(queryId, 'NOT_A_REAL_STATUS'));

      const connection = await createLiveConnection(wiremock.connectionOptions);
      expect(await connection[method](queryId)).toBe(
        isRunningNewDriverWithBD('BD#44') ? 'NO_DATA' : 'NOT_A_REAL_STATUS',
      );
    });

    it('should reject when monitoring returns success false', async () => {
      await wiremock.stub(
        monitoringQueryFailure(queryId, 200, {
          success: false,
          code: '002003',
          message: 'SQL compilation error: monitoring failed for test',
        }),
      );

      const connection = await createLiveConnection(wiremock.connectionOptions);
      await expect(connection[method](queryId)).rejects.toMatchObject({
        name: 'OperationFailedError',
        code: '002003',
        message: 'SQL compilation error: monitoring failed for test',
      });
    });

    it('should reject when monitoring returns HTTP 500', async () => {
      await wiremock.stub(
        monitoringQueryFailure(queryId, 500, {
          success: false,
          message: 'Internal server error',
        }),
      );

      const connection = await createLiveConnection(wiremock.connectionOptions);
      await expect(connection[method](queryId)).rejects.toMatchObject(
        isRunningNewDriverWithBD('BD#46')
          ? {
              name: 'Error',
              message: 'HTTP request failed after retries: query status',
            }
          : {
              name: 'RequestFailedError',
              code: 401002,
              message: 'Request to Snowflake failed.',
            },
      );
    });

    it('should reject when monitoring returns HTTP 400', async () => {
      await wiremock.stub(
        monitoringQueryFailure(queryId, 400, {
          success: false,
          code: '002003',
          message: 'SQL compilation error: monitoring failed for test',
        }),
      );

      const connection = await createLiveConnection(wiremock.connectionOptions);
      await expect(connection[method](queryId)).rejects.toMatchObject(
        isRunningNewDriverWithBD('BD#46')
          ? {
              name: 'Error',
              message: 'Invalid Snowflake response',
            }
          : {
              name: 'RequestFailedError',
              code: 401002,
              message: 'Request to Snowflake failed.',
            },
      );
    });
  },
);
