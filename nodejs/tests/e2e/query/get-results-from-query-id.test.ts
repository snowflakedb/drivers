import { afterAll, beforeAll, beforeEach, describe, expect, it } from 'vitest';
import type { Connection, RowStatement, SnowflakeError } from '../../types/sdk-types.js';
import { createLiveConnection } from '../utils/fixtures.js';
import {
  baseConnectionOptions,
  destroyConnectionAsync,
  executeAsync,
  expectColumnsNames,
  collectStreamedRows,
  getResultFromQueryIdForTest,
  getResultsFromQueryIdAsync,
  IS_RUNNING_FOR_OLD_DRIVER,
  isRunningNewDriverWithBD,
} from '../utils/index.js';
import {
  monitoringQueryStatus,
  monitoringQueryStatuses,
  proxyAllTo,
  WiremockServer,
} from '../utils/wiremock/index.js';

describe('getResultsFromQueryId', () => {
  let connection: Connection;

  beforeAll(async () => {
    connection = await createLiveConnection({}, false);
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  it('should reject with 460001 for a malformed query id', async () => {
    await expect(
      connection.getResultsFromQueryId({ queryId: 'fakeQueryId' }),
    ).rejects.toMatchObject({
      name: 'InvalidParameterError',
      code: 460001,
      message: 'Invalid queryId: fakeQueryId',
    } satisfies Partial<SnowflakeError>);
  });

  it('should return rows and column names for a completed query via stream rows', async () => {
    const { statement: executedStatement, rows: expectedRows } = await executeAsync(
      connection,
      "SELECT 1 AS a, 'hello' AS b",
    );
    const queryId = executedStatement.getQueryId()!;

    const { statement } = await getResultsFromQueryIdAsync(connection, {
      queryId,
      streamResult: true,
    });
    const rows = await collectStreamedRows(statement as RowStatement);

    expect(rows).toEqual(expectedRows);
    expectColumnsNames(statement as RowStatement, ['A', 'B']);
  });

  it('should return rows and column names for a completed query via complete callback', async () => {
    const { statement: executedStatement, rows: expectedRows } = await executeAsync(
      connection,
      "SELECT 1 AS a, 'hello' AS b",
    );
    const queryId = executedStatement.getQueryId()!;

    const { statement, rows } = await getResultsFromQueryIdAsync(connection, { queryId });

    expect(rows).toEqual(expectedRows);
    expectColumnsNames(statement as RowStatement, ['A', 'B']);
  });

  it('should reject with OperationFailedError for a query that failed server-side', async () => {
    const queryId = await executeAsync(
      connection,
      'SELECT * FROM NON_EXISTENT_TABLE_TEST_12345',
    ).then(
      () => {
        throw new Error('expected execute to fail');
      },
      (err: { statement: RowStatement }) => err.statement.getQueryId()!,
    );

    await expect(connection.getResultsFromQueryId({ queryId })).rejects.toMatchObject({
      name: 'OperationFailedError',
    } satisfies Partial<SnowflakeError>);
  });

  it('should reject with 460002 for a valid UUID that does not exist', async () => {
    const queryId = '12345678-1234-4123-A123-123456789012';
    await expect(getResultFromQueryIdForTest(connection, { queryId })).rejects.toMatchObject({
      name: 'ClientError',
      code: 460002,
      message: isRunningNewDriverWithBD('BD#43')
        ? `Cannot retrieve data. No information returned from server for query ${queryId}`
        : `Cannot retrieve data. No information returned from server for query ${queryId} NO_QUERY_DATA`,
    } satisfies Partial<SnowflakeError>);
  });
});

describe('getResultsFromQueryId with Wiremock', () => {
  let wiremock: WiremockServer;
  let connection: Connection;

  beforeAll(async () => {
    wiremock = await WiremockServer.spawn();
    await wiremock.stub(proxyAllTo(`https://${baseConnectionOptions.host}`));
    connection = await createLiveConnection(wiremock.connectionOptions, false);
  });

  beforeEach(async () => {
    await wiremock.reset();
    await wiremock.stub(proxyAllTo(`https://${baseConnectionOptions.host}`));
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
    await wiremock.destroy();
  });

  it('should return rows after polling RUNNING then SUCCESS', async () => {
    const { statement: executedStatement, rows: expectedRows } = await executeAsync(
      connection,
      "SELECT 1 AS a, 'hello' AS b",
    );
    const queryId = executedStatement.getQueryId()!;
    await wiremock.stub(monitoringQueryStatuses(queryId, ['RUNNING', 'RUNNING', 'SUCCESS']));

    const { statement, rows } = await getResultsFromQueryIdAsync(connection, { queryId });

    expect(rows).toEqual(expectedRows);
    expectColumnsNames(statement as RowStatement, ['A', 'B']);
  });

  // old driver doesn't allow to configure sleep time between retries, so we skip this test for it
  // it would take too long to run
  it.skipIf(IS_RUNNING_FOR_OLD_DRIVER)(
    'should reject with 460002 after the NO_DATA retry cap',
    async () => {
      const queryId = '12345678-1234-4123-a123-123456789012';
      await wiremock.stub(monitoringQueryStatus(queryId, 'NO_DATA'));

      await expect(getResultFromQueryIdForTest(connection, { queryId })).rejects.toMatchObject({
        name: 'ClientError',
        code: 460002,
        message: `Cannot retrieve data. No information returned from server for query ${queryId}`,
      } satisfies Partial<SnowflakeError>);
    },
  );

  it('should reject with 460003 when monitoring reports RESTARTED', async () => {
    const queryId = '12345678-1234-4123-a123-123456789012';
    await wiremock.stub(monitoringQueryStatus(queryId, 'RESTARTED'));

    await expect(getResultFromQueryIdForTest(connection, { queryId })).rejects.toMatchObject({
      name: 'ClientError',
      code: 460003,
      message: `Status of query ${queryId} is RESTARTED, results are unavailable`,
    } satisfies Partial<SnowflakeError>);
  });
});
