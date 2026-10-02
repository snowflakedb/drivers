const SfTimestamp = require('./../../../../lib/connection/result/sf_timestamp');
const assert = require('assert');

describe('Date: basic', function () {
  const testCases = [
    {
      name: 'date: YYYY-MM-DD',
      options: {
        epochSeconds: 1448496000,
        nanoSeconds: 0,
        scale: 0,
        timezone: 'UTC',
        format: 'YYYY-MM-DD',
      },
      result: '2015-11-26',
    },
    {
      name: 'date: YYYY',
      options: {
        epochSeconds: 1448496000,
        nanoSeconds: 0,
        scale: 0,
        timezone: 'UTC',
        format: 'YYYY',
      },
      result: '2015',
    },
    {
      name: 'date: MM',
      options: {
        epochSeconds: 1448496000,
        nanoSeconds: 0,
        scale: 0,
        timezone: 'UTC',
        format: 'MM',
      },
      result: '11',
    },
    {
      name: 'date: DD',
      options: {
        epochSeconds: 1448496000,
        nanoSeconds: 0,
        scale: 0,
        timezone: 'UTC',
        format: 'DD',
      },
      result: '26',
    },
  ];

  testCases.forEach(function (testCase) {
    it(testCase.name, function () {
      const options = testCase.options;
      assert.strictEqual(
        new SfTimestamp(
          options.epochSeconds,
          options.nanoSeconds,
          options.scale,
          options.timezone,
          options.format,
        ).toString(),
        testCase.result,
      );
    });
  });
});

// TODO: Time: basic → time_basic_format_matrix_from_legacy_sf_timestamp in time_format.rs.
