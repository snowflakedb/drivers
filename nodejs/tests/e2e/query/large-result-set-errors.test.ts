import { afterAll, beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { createLiveConnection } from '../utils/fixtures.js';
import {
  executeAsync,
  isRunningNewDriverWithBD,
  NOT_IMPLEMENTED_IN_NEW_DRIVER,
  patchOldDriverChunkRetrySleepCap,
} from '../utils/index.js';
import {
  jsonResponse,
  loginSuccess,
  logoutSuccess,
  type ResponseDefinition,
  resultWithOneRemoteChunk,
  WiremockServer,
} from '../utils/wiremock/index.js';

const QUERY_ID = '01baf79b-0108-1a60-0000-01110354a6cf';
const CHUNK_PATH = '/chunks/0';
const FRESH_CHUNK_PATH = '/chunks/0-fresh';

describe('Large Result Set Errors', () => {
  let wiremock: WiremockServer;

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

  async function stubResultWithOneRemoteChunk(chunkResponse: ResponseDefinition): Promise<void> {
    await wiremock.stub({
      request: { method: 'POST', urlPathPattern: '/queries/v1/query-request.*' },
      response: resultWithOneRemoteChunk(QUERY_ID, `${wiremock.url}${CHUNK_PATH}`),
    });
    await wiremock.stub({
      request: { method: 'GET', urlPath: CHUNK_PATH },
      response: chunkResponse,
    });
  }

  // Neither driver retries 404 or treats it as an expired chunk URL.
  it('should report a request failure when cloud storage answers a chunk download with an error status', async () => {
    await stubResultWithOneRemoteChunk(jsonResponse(404, {}));
    const connection = await createLiveConnection(wiremock.connectionOptions);

    await expect(executeAsync(connection, 'select 1')).rejects.toMatchObject({
      error: {
        code: 402002,
        message: 'Request to S3/Blob failed.',
        response: { statusCode: 404, statusText: 'Not Found' },
        ...(isRunningNewDriverWithBD('BD#130')
          ? { name: 'RequestFailedError', cause: { message: 'HTTP status 404 Not Found' } }
          : { name: 'LargeResultSetError' }),
      },
    });
  });

  it('should fetch the result again for fresh chunk URLs when cloud storage answers a chunk download with 403', async () => {
    patchOldDriverChunkRetrySleepCap(0);
    await stubResultWithOneRemoteChunk(jsonResponse(403, {}));
    await wiremock.stub({
      request: { method: 'GET', urlPath: `/queries/${QUERY_ID}/result` },
      response: resultWithOneRemoteChunk(QUERY_ID, `${wiremock.url}${FRESH_CHUNK_PATH}`),
    });
    await wiremock.stub({
      request: { method: 'GET', urlPath: FRESH_CHUNK_PATH },
      response: { status: 200, body: '["1"]' },
    });
    const connection = await createLiveConnection(wiremock.connectionOptions);

    if (isRunningNewDriverWithBD('BD#131')) {
      await expect(executeAsync(connection, 'select 1')).rejects.toMatchObject({
        error: {
          name: 'RequestFailedError',
          code: 402002,
          message: 'Request to S3/Blob failed.',
          response: { statusCode: 403, statusText: 'Forbidden' },
          cause: { message: 'HTTP status 403 Forbidden' },
        },
      });
      expect(await wiremock.findRequests(`/queries/${QUERY_ID}/result`)).toHaveLength(0);
    } else {
      const { rows } = await executeAsync(connection, 'select 1');
      expect(rows).toEqual([{ C1: '1' }]);
      expect(await wiremock.findRequests(`/queries/${QUERY_ID}/result`)).toHaveLength(1);
    }
  });

  // TODO(SNOW-4334410): the new driver reads this chunk as zero rows and the query succeeds.
  it.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)(
    'should report a request failure when cloud storage answers a chunk download with an XML document',
    async () => {
      await stubResultWithOneRemoteChunk({
        status: 200,
        headers: { 'Content-Type': 'application/xml' },
        body: '<?xml version="1.0" encoding="UTF-8"?><Error><Code>AccessDenied</Code><Message>Request has expired</Message></Error>',
      });
      const connection = await createLiveConnection(wiremock.connectionOptions);

      await expect(executeAsync(connection, 'select 1')).rejects.toMatchObject({
        error: {
          code: 402002,
          message: 'Request to S3/Blob failed.',
          ...(isRunningNewDriverWithBD('BD#130')
            ? { name: 'RequestFailedError' }
            : {
                name: 'LargeResultSetError',
                response: expect.objectContaining({ statusCode: 200 }),
              }),
        },
      });
    },
  );

  it('should report NetworkError when cloud storage cannot be reached for a chunk download', async () => {
    patchOldDriverChunkRetrySleepCap(0);
    await stubResultWithOneRemoteChunk({ fault: 'CONNECTION_RESET_BY_PEER' });
    const connection = await createLiveConnection(wiremock.connectionOptions);

    await expect(executeAsync(connection, 'select 1')).rejects.toMatchObject({
      error: {
        name: 'NetworkError',
        code: 402001,
        message: isRunningNewDriverWithBD('BD#79')
          ? 'Network error. Could not reach S3/Blob.'
          : expect.stringMatching(/ECONNRESET/),
      },
    });
  });
});
