import { describe, it, beforeAll, beforeEach, afterAll, expect } from 'vitest';
import type {
  Connection,
  QueryStatus,
  RowStatement,
  SnowflakeError,
} from '../../types/sdk-types.js';
import { createLiveConnection } from '../utils/fixtures.js';
import {
  cancelStatementAsync,
  connectionGetQueryStatusWithBD,
  destroyConnectionAsync,
  executeAsync,
  isRunningNewDriverWithBD,
  randomizeName,
  sendExecute,
  sleepAsync,
} from '../utils/index.js';
import {
  jsonResponse,
  loginSuccess,
  logoutSuccess,
  WiremockServer,
} from '../utils/wiremock/index.js';

const NOT_EXECUTING_CODE = '000605';
const POLL_MS = 250;
const STATUS_DEADLINE_MS = 30_000;
const QUERY_REQUEST_DELAY_MS = 15_000;
const ABORT_DELAY_MS = 2_000;
const CANCELED_EXECUTE = {
  name: 'OperationFailedError',
  code: '000604',
  sqlState: '57014',
  message: 'SQL execution canceled',
};

async function waitForStatus(
  connection: Connection,
  queryId: string,
  accept: (status: QueryStatus) => boolean,
): Promise<QueryStatus> {
  const deadline = Date.now() + STATUS_DEADLINE_MS;
  let status: QueryStatus | undefined;
  while (Date.now() < deadline) {
    status = await connectionGetQueryStatusWithBD(connection, queryId);
    if (accept(status)) {
      return status;
    }
    await sleepAsync(POLL_MS);
  }
  throw new Error(`query ${queryId} status stayed ${status ?? 'unknown'}`);
}

async function waitForRunningQueryId(connection: Connection, queryTag: string): Promise<string> {
  const deadline = Date.now() + STATUS_DEADLINE_MS;
  while (Date.now() < deadline) {
    const { rows } = await executeAsync(
      connection,
      `select query_id from table(information_schema.query_history_by_session())
       where execution_status = 'RUNNING' and query_tag = ?`,
      { binds: [queryTag] },
    );
    if (rows.length > 0) {
      return rows[0].QUERY_ID as string;
    }
    await sleepAsync(POLL_MS);
  }
  throw new Error(`query tagged ${queryTag} did not start running`);
}

describe('Query Cancellation', () => {
  let connection: Connection;

  beforeAll(async () => {
    connection = await createLiveConnection({}, false);
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  it('should cancel a running sync query when the server accepts the abort', async () => {
    const queryTag = randomizeName('CANCEL');
    const { statement, completion } = sendExecute(
      connection,
      'select count(*) from table(generator(timeLimit => 60))',
      { parameters: { QUERY_TAG: queryTag } },
    );
    const failure = completion.then(
      () => undefined,
      (rejection: { error: SnowflakeError }) => rejection.error,
    );
    const queryId = await waitForRunningQueryId(connection, queryTag);

    await cancelStatementAsync(statement as RowStatement);

    expect(await failure).toMatchObject(CANCELED_EXECUTE);
    expect(
      await waitForStatus(connection, queryId, (status) => !connection.isStillRunning(status)),
    ).toBe('FAILED_WITH_ERROR');
  });

  it('should cancel a finished query when the server says it is not running', async () => {
    const { statement } = await executeAsync(connection, 'select 1');
    if (isRunningNewDriverWithBD('BD#116')) {
      await cancelStatementAsync(statement);
      expect(await connectionGetQueryStatusWithBD(connection, statement.getQueryId()!)).toBe(
        'SUCCESS',
      );
    } else {
      await expect(cancelStatementAsync(statement)).rejects.toMatchObject({
        name: 'OperationFailedError',
        code: NOT_EXECUTING_CODE,
        sqlState: '01000',
      } satisfies Partial<SnowflakeError>);
    }
  });

  it('should cancel a running async query by query id when the server accepts the abort', async () => {
    const queryTag = randomizeName('CANCEL_ASYNC');
    const { statement } = await executeAsync(
      connection,
      'select count(*) from table(generator(timeLimit => 60))',
      { asyncExec: true, parameters: { QUERY_TAG: queryTag } },
    );
    const queryId = await waitForRunningQueryId(connection, queryTag);

    await cancelStatementAsync(statement as RowStatement);

    expect(
      await waitForStatus(connection, queryId, (status) => !connection.isStillRunning(status)),
    ).toBe('FAILED_WITH_ERROR');
  });
});

describe('Query Cancellation with Wiremock', () => {
  const queryId = '01baf79b-0108-1a60-0000-01110354a6ce';
  let wiremock: WiremockServer;

  beforeAll(async () => {
    wiremock = await WiremockServer.spawn();
  });

  beforeEach(async () => {
    await wiremock.reset();
    await wiremock.stub(loginSuccess());
    await wiremock.stub(logoutSuccess());
    await wiremock.stub({
      request: {
        method: 'POST',
        urlPathPattern: '/queries/v1/query-request.*',
      },
      response: jsonResponse(200, {
        success: true,
        code: '333333',
        data: { queryId },
      }),
    });
  });

  afterAll(async () => {
    await wiremock.destroy();
  });

  it('should cancel an async query by query id when the server rejects the abort', async () => {
    await wiremock.stub({
      request: {
        method: 'POST',
        urlPathPattern: `/queries/${queryId}/abort-request.*`,
      },
      response: {
        ...jsonResponse(403, {
          success: false,
          code: null,
          message: null,
          data: null,
        }),
        fixedDelayMilliseconds: ABORT_DELAY_MS,
      },
    });
    const connection = await createLiveConnection(wiremock.connectionOptions);
    const { statement } = await executeAsync(connection, 'select 1', {
      asyncExec: true,
    });

    if (isRunningNewDriverWithBD('BD#116')) {
      const started = Date.now();
      await cancelStatementAsync(statement);
      expect(Date.now() - started).toBeLessThan(ABORT_DELAY_MS);
    } else {
      await expect(cancelStatementAsync(statement)).rejects.toMatchObject({
        name: 'RequestFailedError',
        code: 401002,
      });
    }
  });

  async function holdQueryRequest(): Promise<void> {
    await wiremock.stub({
      priority: 1,
      request: {
        method: 'POST',
        urlPathPattern: '/queries/v1/query-request.*',
      },
      response: {
        ...jsonResponse(200, {
          success: true,
          code: '333333',
          data: { queryId },
        }),
        fixedDelayMilliseconds: QUERY_REQUEST_DELAY_MS,
      },
    });
  }

  async function stubRequestIdAbort(status: number, success: boolean): Promise<void> {
    await wiremock.stub({
      request: {
        method: 'POST',
        urlPathPattern: '/queries/v1/abort-request.*',
      },
      response: {
        ...jsonResponse(status, {
          success,
          code: null,
          message: null,
          data: null,
        }),
        fixedDelayMilliseconds: ABORT_DELAY_MS,
      },
    });
  }

  async function startBeforeQueryId(asyncExec: boolean): Promise<{
    statement: RowStatement;
    failure: Promise<SnowflakeError | undefined>;
  }> {
    const connection = await createLiveConnection(wiremock.connectionOptions);
    const { statement, completion } = sendExecute(
      connection,
      'select 1',
      asyncExec ? { asyncExec: true } : {},
    );
    await expect
      .poll(async () => (await wiremock.findRequests('/queries/v1/query-request.*')).length, {
        interval: 50,
        timeout: 2500,
      })
      .toBeGreaterThan(0);
    return {
      statement: statement as RowStatement,
      failure: completion.then(
        () => undefined,
        (rejection: { error: SnowflakeError }) => rejection.error,
      ),
    };
  }

  it('should cancel an async query before it has a query id when the server accepts the abort', async () => {
    await holdQueryRequest();
    await stubRequestIdAbort(200, true);
    const { statement, failure } = await startBeforeQueryId(true);

    if (isRunningNewDriverWithBD('BD#116')) {
      const started = Date.now();
      await cancelStatementAsync(statement);
      expect(Date.now() - started).toBeLessThan(ABORT_DELAY_MS);
      expect(await failure).toMatchObject(CANCELED_EXECUTE);
    } else {
      await cancelStatementAsync(statement);
      await failure;
    }
  });

  it('should cancel a running sync query when the server rejects the abort', async () => {
    await holdQueryRequest();
    await stubRequestIdAbort(403, false);
    const { statement, failure } = await startBeforeQueryId(false);

    if (isRunningNewDriverWithBD('BD#116')) {
      const started = Date.now();
      await cancelStatementAsync(statement);
      expect(Date.now() - started).toBeLessThan(ABORT_DELAY_MS);
      expect(await failure).toMatchObject(CANCELED_EXECUTE);
    } else {
      await expect(cancelStatementAsync(statement)).rejects.toMatchObject({
        name: 'RequestFailedError',
        code: 401002,
      });
      await failure;
    }
  });

  it('should cancel an async query before it has a query id when the server rejects the abort', async () => {
    await holdQueryRequest();
    await stubRequestIdAbort(403, false);
    const { statement, failure } = await startBeforeQueryId(true);

    if (isRunningNewDriverWithBD('BD#116')) {
      const started = Date.now();
      await cancelStatementAsync(statement);
      expect(Date.now() - started).toBeLessThan(ABORT_DELAY_MS);
      expect(await failure).toMatchObject(CANCELED_EXECUTE);
    } else {
      await expect(cancelStatementAsync(statement)).rejects.toMatchObject({
        name: 'RequestFailedError',
        code: 401002,
      });
      await failure;
    }
  });
});
