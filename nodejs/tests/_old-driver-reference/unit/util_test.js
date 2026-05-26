const Util = require('./../../lib/util');
const assert = require('assert');

describe('Util', function () {
  it('Util.number.isNonNegativeInteger()', function () {
    // positive tests
    assert.ok(Util.number.isNonNegativeInteger(0));
    assert.ok(Util.number.isNonNegativeInteger(1));
    assert.ok(Util.number.isNonNegativeInteger(100));
    assert.ok(Util.number.isNonNegativeInteger(Number.MAX_SAFE_INTEGER));
    assert.ok(Util.number.isNonNegativeInteger(Number.MAX_VALUE));

    // negative tests
    assert.ok(!Util.number.isNonNegativeInteger(Number.MIN_VALUE));
    assert.ok(!Util.number.isNonNegativeInteger(1.1));
    assert.ok(!Util.number.isNonNegativeInteger(-1.1));
    assert.ok(!Util.number.isNonNegativeInteger(Number.MIN_SAFE_INTEGER));
    assert.ok(!Util.number.isNonNegativeInteger(Number.POSITIVE_INFINITY));
    assert.ok(!Util.number.isNonNegativeInteger(Number.NEGATIVE_INFINITY));
  });

  describe('Util.isLoginRequest Test', function () {
    const baseUrl = 'wwww.test.com';
    const testCases = [
      {
        testName: 'test URL with a right login end point',
        endPoint: '/v1/login-request',
        result: true,
      },
      {
        testName: 'test URL with a wrong login end point',
        endPoint: '/login-request',
        result: false,
      },
      {
        testName: 'test URL with a right authenticator-request point',
        endPoint: '/authenticator-request',
        result: true,
      },
      {
        testName: 'test URL with a wrong authenticator-request point',
        endPoint: '/authenticator-requ',
        result: false,
      },
    ];

    for (const { testName, endPoint, result } of testCases) {
      it(testName, function () {
        const isLoginRequest = Util.isLoginRequest(baseUrl + endPoint);
        assert.strictEqual(isLoginRequest, result);
      });
    }
  });

  describe('Util.getJitterSleepTime Test', function () {
    it('test - retryTimeout is over 300', function () {
      const errorCodes = [
        {
          statusCode: 403,
          retry403: true,
          isRetryable: true,
        },
        {
          statusCode: 408,
          retry403: false,
          isRetryable: true,
        },
        {
          statusCode: 429,
          retry403: false,
          isRetryable: true,
        },
        {
          statusCode: 500,
          retry403: false,
          isRetryable: true,
        },
        {
          statusCode: 503,
          retry403: false,
          isRetryable: true,
        },
        {
          statusCode: 538,
          retry403: false,
          isRetryable: true,
        },
      ];

      const maxRetryTimeout = 300;
      let currentSleepTime = 1;
      let retryCount = 0;
      let totalElapsedTime = currentSleepTime;
      for (const response of errorCodes) {
        const result = Util.getJitteredSleepTime(
          retryCount,
          currentSleepTime,
          totalElapsedTime,
          maxRetryTimeout,
        );
        const jitter = currentSleepTime / 2;
        const nextSleep = 2 ** retryCount;
        currentSleepTime = result.sleep;
        totalElapsedTime = result.totalElapsedTime;
        retryCount++;

        assert.strictEqual(Util.isRetryableHttpError(response, true), true);
        assert.ok(currentSleepTime <= nextSleep + jitter || currentSleepTime >= nextSleep - jitter);
      }

      assert.strictEqual(retryCount, 6);
      assert.ok(totalElapsedTime <= maxRetryTimeout);
    });

    it('test - retryTimeout is 0', function () {
      const maxRetryTimeout = 0;
      let currentSleepTime = 1;
      const maxRetryCount = 20;
      let totalElapsedTime = currentSleepTime;
      let retryCount = 1;
      for (; retryCount < maxRetryCount; retryCount++) {
        const result = Util.getJitteredSleepTime(
          retryCount,
          currentSleepTime,
          totalElapsedTime,
          maxRetryTimeout,
        );
        const jitter = currentSleepTime / 2;
        const nextSleep = 2 ** retryCount;
        currentSleepTime = result.sleep;
        totalElapsedTime = result.totalElapsedTime;

        assert.ok(currentSleepTime <= nextSleep + jitter || currentSleepTime >= nextSleep - jitter);
      }

      assert.strictEqual(retryCount, 20);
    });
  });

  it('Util.getJitter Test', function () {
    const randomNumber = Util.chooseRandom(10, 100);
    const jitter = Util.getJitter(randomNumber);

    assert.ok(randomNumber / -2 <= jitter && jitter <= randomNumber / 2);
  });

  it('Util.isRetryableHttpError()', function () {
    const testCasesPos = [
      {
        name: '200 - OK',
        statusCode: 200,
        retry403: false,
        isRetryable: false,
      },
      {
        name: '400 - Bad Request',
        statusCode: 400,
        retry403: false,
        isRetryable: false,
      },
      {
        name: '403 - Forbidden',
        statusCode: 403,
        retry403: false,
        isRetryable: false,
      },
      {
        name: '403 - Forbidden (retry on 403)',
        statusCode: 403,
        retry403: true,
        isRetryable: true,
      },
      {
        name: '404 - Not Found',
        statusCode: 404,
        retry403: false,
        isRetryable: false,
      },
      {
        name: '408 - Request Timeout',
        statusCode: 408,
        retry403: false,
        isRetryable: true,
      },
      {
        name: '429 - Too Many Requests',
        statusCode: 429,
        retry403: false,
        isRetryable: true,
      },
      {
        name: '500 - Internal Server Error',
        statusCode: 500,
        retry403: false,
        isRetryable: true,
      },
      {
        name: '503 - Service Unavailable',
        statusCode: 503,
        retry403: false,
        isRetryable: true,
      },
    ];

    let testCase;
    let err;
    for (let index = 0, length = testCasesPos.length; index < length; index++) {
      testCase = testCasesPos[index];
      err = {
        response: { statusCode: testCase.statusCode },
      };
      assert.strictEqual(
        Util.isRetryableHttpError(err.response, testCase.retry403),
        testCase.isRetryable,
      );
    }
  });

  describe('Okta Authentication Retry Condition', () => {
    const testCases = [
      {
        name: 'test - default values',
        retryOption: {
          maxRetryCount: 7,
          numRetries: 1,
          remainingTimeout: 300000,
          maxRetryTimeout: 300000,
        },
        result: true,
      },
      {
        name: 'test - the value of the numRetries is the same as the max retry count',
        retryOption: {
          maxRetryCount: 7,
          numRetries: 7,
          remainingTimeout: 300000,
          maxRetryTimeout: 300000,
        },
        result: true,
      },
      {
        name: 'test - max retry timeout is 0',
        retryOption: {
          maxRetryCount: 7,
          numRetries: 1,
          remainingTimeout: 300000,
          maxRetryTimeout: 0,
        },
        result: true,
      },
      {
        name: 'test - the max retry timeout is 0 and number of retry is over',
        retryOption: {
          maxRetryCount: 7,
          numRetries: 8,
          remainingTimeout: -50,
          maxRetryTimeout: 0,
        },
        result: false,
      },
      {
        name: 'test - the retry count is over the max retry count ',
        retryOption: {
          maxRetryCount: 7,
          numRetries: 8,
          remainingTimeout: 300000,
          maxRetryTimeout: 300,
        },
        result: false,
      },
      {
        name: 'test - the remaining timeout is 0',
        retryOption: {
          maxRetryCount: 7,
          numRetries: 8,
          remainingTimeout: 0,
          maxRetryTimeout: 300,
        },
        result: false,
      },
      {
        name: 'test - the remaining timeout is negative',
        retryOption: {
          maxRetryCount: 7,
          numRetries: 8,
          remainingTimeout: -10,
          maxRetryTimeout: 300,
        },
        result: false,
      },
    ];

    testCases.forEach(({ name, retryOption, result }) => {
      it(name, () => {
        assert.strictEqual(
          Util.shouldRetryOktaAuth({ ...retryOption, startTime: Date.now() }),
          result,
        );
      });
    });
  });

  describe('isPrivateKey', () => {
    [
      // pragma: allowlist nextline secret
      {
        name: 'trimmed already key',
        key: '-----BEGIN PRIVATE KEY-----\ntest\n-----END PRIVATE KEY-----',
      },
      {
        name: 'key with whitespaces at the beginning',
        // pragma: allowlist nextline secret
        key: '   -----BEGIN PRIVATE KEY-----\ntest\n-----END PRIVATE KEY-----',
      },
      {
        name: 'key with whitespaces at the end',
        // pragma: allowlist nextline secret
        key: '-----BEGIN PRIVATE KEY-----\ntest\n-----END PRIVATE KEY-----\n\n\n',
      },
    ].forEach(({ name, key }) => {
      it(`${name} is valid`, () => {
        assert.ok(Util.isPrivateKey(key));
      });
    });

    [
      { name: 'key without beginning and end', key: 'test' },
      { name: 'key with missing beginning', key: 'test\n-----END PRIVATE KEY-----' },
      {
        name: 'key with missing ending',
        // pragma: allowlist nextline secret
        key: '   -----BEGIN PRIVATE KEY-----\ntest',
      },
      {
        name: 'key with invalid beginning',
        key: '-----BEGIN PUBLIC KEY-----\ntest\n-----END PRIVATE KEY-----\n\n\n',
      },
      {
        name: 'key with invalid end',
        // pragma: allowlist nextline secret
        key: '-----BEGIN PRIVATE KEY-----\ntest\n-----END PUBLIC KEY-----\n\n\n',
      },
    ].forEach(({ name, key }) => {
      it(`${name} is invalid`, () => {
        assert.ok(!Util.isPrivateKey(key));
      });
    });
  });

  describe('isPrivateLink', () => {
    [
      {
        name: 'private link',
        host: 'account.privatelink.snowflakecomputing.com',
        result: true,
      },
      {
        name: 'private link upper case letters',
        host: 'ACCOUNT.PRIVATELINK.SNOWFLAKECOMPUTING.COM',
        result: true,
      },
      {
        name: 'private link mixed case letters',
        host: 'account.privateLINK.snowflakecomputING.com',
        result: true,
      },
      {
        name: 'no private link',
        host: 'account.snowflakecomputing.com',
        result: false,
      },
      {
        name: 'private link cn',
        host: 'account.privatelink.snowflakecomputing.cn',
        result: true,
      },
      {
        name: 'no private link cn',
        host: 'account.snowflakecomputing.cn',
        result: false,
      },
    ].forEach(({ name, host, result }) => {
      it(`${name} is valid`, () => {
        assert.equal(Util.isPrivateLink(host), result);
      });
    });
  });
});
