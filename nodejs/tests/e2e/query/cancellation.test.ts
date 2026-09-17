import { describe, it, beforeAll, afterAll, expect } from 'vitest';
import type { Connection, RowStatement, SnowflakeError } from '../../types/sdk-types.js';
import {
  createTestConnection,
  destroyConnectionAsync,
  executeAsync,
  NOT_IMPLEMENTED_IN_NEW_DRIVER,
  sleepAsync,
} from '../utils/index.js';

const NOT_EXECUTING_CODE = '000605';
const POLL_MS = 250;
const CANCEL_START_DEADLINE_MS = 30_000;

function cancelStatement(statement: RowStatement) {
  return new Promise<void>((resolve, reject) => {
    statement.cancel((err) => (err ? reject(err) : resolve()));
  });
}

function isNotCurrentlyExecuting(err: unknown): boolean {
  return (err as SnowflakeError).code === NOT_EXECUTING_CODE;
}

// Cancel uses requestId until GS returns a query id (old SDK: getQueryId() stays
// empty while the query HTTP request is still open). Retry 000605 until the
// generator is executing. Do not treat 000605 as success.
async function cancelOnceExecuting(statement: RowStatement): Promise<void> {
  const deadline = Date.now() + CANCEL_START_DEADLINE_MS;
  let lastError: unknown;
  while (Date.now() < deadline) {
    try {
      await cancelStatement(statement);
      return;
    } catch (err) {
      if (!isNotCurrentlyExecuting(err)) {
        throw err;
      }
      lastError = err;
      await sleepAsync(POLL_MS);
    }
  }
  if (lastError) {
    throw lastError;
  }
  throw new Error(`query was not executing in time to cancel (${CANCEL_START_DEADLINE_MS}ms)`);
}

describe('Query Cancellation', () => {
  let connection: Connection;

  beforeAll(async () => {
    connection = createTestConnection();
    await connection.connectAsync();
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  it('cancels a running query', async () => {
    const statement = connection.execute({
      sqlText: 'select count(*) from table(generator(timeLimit => 3600))',
    });
    await cancelOnceExecuting(statement);
  });

  // TODO: We need to have 2 tests
  // 1. A wiremock that simulates network failure - it should throw
  // 2. A BD that 000605 "Identified SQL statement is not currently executing" is a silient no-op
  //    instead of failure
  // Also maybe should check query status from server after the cancelation
  it.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)('throws when failing to cancel a query', async () => {
    const { statement } = await executeAsync(connection, 'select 1');
    // Query is completed = nothing to cancel
    await expect(cancelStatement(statement)).rejects.toMatchObject({
      code: NOT_EXECUTING_CODE,
      sqlState: '01000',
    });
  });
});
