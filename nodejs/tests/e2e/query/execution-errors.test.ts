import { describe, it, beforeAll, afterAll, expect, beforeEach } from 'vitest';
import type { Connection, SnowflakeError } from '../../types/sdk-types.js';
import { createLiveConnection } from '../utils/fixtures.js';
import { destroyConnectionAsync, executeAsync, isRunningNewDriverWithBD } from '../utils/index.js';
import {
  loginSuccess,
  logoutSuccess,
  queryRequestFail,
  WiremockServer,
  withResponse,
} from '../utils/wiremock/index.js';

describe('Execution Errors', () => {
  let connection: Connection;

  beforeAll(async () => {
    connection = await createLiveConnection({}, false);
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  it('should throw OperationFailedError for a query that fails server-side', async () => {
    const execution = executeAsync(connection, 'select * from a_table_that_does_not_exist');
    await expect(execution).rejects.toMatchObject({
      error: {
        name: 'OperationFailedError',
        code: '002003',
        sqlState: '42S02',
        message: expect.stringContaining('does not exist or not authorized'),
      } satisfies Partial<SnowflakeError>,
    });
  });
});

describe('Execution Errors with Wiremock', () => {
  let wiremock: WiremockServer;

  beforeAll(async () => {
    wiremock = await WiremockServer.spawn();
  });

  beforeEach(async () => {
    await wiremock.reset();
    await wiremock.stub([loginSuccess(), logoutSuccess()]);
  });

  afterAll(async () => {
    await wiremock.destroy();
  });

  it('should reject a query when Snowflake answers with HTTP 400', async () => {
    await wiremock.stub(withResponse(queryRequestFail(), { status: 400 }));
    const connection = await createLiveConnection(wiremock.connectionOptions);
    await expect(executeAsync(connection, 'SELECT 1')).rejects.toMatchObject({
      error: {
        name: 'RequestFailedError',
        code: 401002,
        message: 'Request to Snowflake failed.',
        response: { statusCode: 400, statusText: 'Bad Request' },
        ...(isRunningNewDriverWithBD('BD#46')
          ? {
              cause: {
                message:
                  'Invalid Snowflake response: Snowflake responded with error status: ' +
                  '400 Bad Request, message: Unexpected response, body length 17.',
              },
            }
          : {}),
      },
    });
  });
});
