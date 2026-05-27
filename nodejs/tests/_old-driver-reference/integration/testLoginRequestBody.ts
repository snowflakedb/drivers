import assert from 'assert';
import rewiremock from 'rewiremock/node';
import sinon from 'sinon';
import { WIP_ConnectionOptions } from '../../lib/connection/types';
import axiosInstance from '../../lib/http/axios_instance';
import { getFreePort } from '../../lib/util';
import { runWireMockAsync, addWireMockMappingsFromFile } from '../wiremockRunner';
import * as testUtil from './testUtil';

describe('/login-request body', () => {
  let wiremock: any;
  let connection: any;
  let axiosRequestSpy: sinon.SinonSpy;

  async function initConnection(
    connectionOptions?: Partial<WIP_ConnectionOptions>,
    coreInstance?: any,
  ) {
    connection = testUtil.createConnection(
      {
        accessUrl: wiremock.rootUrl,
        ...connectionOptions,
      },
      coreInstance?.default,
    );
    await testUtil.connectAsync(connection);
  }

  function getLoginRequestData() {
    return axiosRequestSpy.firstCall.firstArg.data.data;
  }

  before(async () => {
    const port = await getFreePort();
    wiremock = await runWireMockAsync(port);
    await addWireMockMappingsFromFile(wiremock, 'wiremock/mappings/login_request_ok.json');
  });

  beforeEach(async () => {
    axiosRequestSpy = sinon.spy(axiosInstance, 'request');
  });

  afterEach(async () => {
    sinon.restore();
  });

  after(async () => {
    await wiremock.global.shutdown();
  });

  describe('CLIENT_ENVIRONMENT', () => {
    function getClientEnvironment() {
      return getLoginRequestData().CLIENT_ENVIRONMENT;
    }

    it('contains APPLICATION if passed in connection config', async () => {
      await initConnection({ application: 'test-application' });
      assert.strictEqual(getClientEnvironment().APPLICATION, 'test-application');
    });

    it('contains APPLICATION_PATH', async () => {
      await initConnection();
      assert.ok(getClientEnvironment().APPLICATION_PATH, 'APPLICATION_PATH should not be empty');
    });

    it('contains instruction set architecture (ISA)', async () => {
      await initConnection();
      assert.strictEqual(getClientEnvironment().ISA, process.arch);
    });

    it('contains OS_DETAILS on Linux or null on other platforms', async () => {
      await initConnection();
      const osDetails = getClientEnvironment().OS_DETAILS;
      if (process.platform === 'linux') {
        assert.ok(
          osDetails !== null && Object.keys(osDetails).length >= 1,
          'OS_DETAILS should contain at least 1 key on Linux',
        );
      } else {
        assert.strictEqual(osDetails, null);
      }
    });

    it('contains LIBC_FAMILY and LIBC_VERSION on Linux or null on other platforms', async () => {
      await initConnection();
      const clientEnvironment = getClientEnvironment();
      if (process.platform === 'linux') {
        assert.ok(
          clientEnvironment.LIBC_FAMILY,
          `LIBC_FAMILY should not be empty on Linux, got: ${clientEnvironment.LIBC_FAMILY}`,
        );
        assert.ok(
          clientEnvironment.LIBC_VERSION,
          `LIBC_VERSION should not be empty on Linux, got: ${clientEnvironment.LIBC_VERSION}`,
        );
      } else {
        assert.strictEqual(clientEnvironment.LIBC_FAMILY, null);
        assert.strictEqual(clientEnvironment.LIBC_VERSION, null);
      }
    });

    it('contains PLATFORM field with mocked lambda env', async () => {
      sinon.stub(process, 'env').value({ ...process.env, LAMBDA_TASK_ROOT: '/var/task' });
      delete require.cache[require.resolve('../../lib/telemetry/platform_detection')];
      const freshCoreInstance = rewiremock.proxy('../../lib/snowflake', {
        '../../lib/telemetry/platform_detection': rewiremock.proxy(
          '../../lib/telemetry/platform_detection',
        ),
      });
      await initConnection({}, freshCoreInstance);
      const platform = getClientEnvironment().PLATFORM;
      assert.ok(Array.isArray(platform), 'PLATFORM should be an array');
      assert.ok(
        platform.includes('is_aws_lambda'),
        `Expected PLATFORM to include is_aws_lambda, got: ${JSON.stringify(platform)}`,
      );
    });
  });
});
