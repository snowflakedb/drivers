const assert = require('assert');

// TODO: Ensure we have unit tests verifying correct conversion of server response values to all Date types.
describe('Result: test timestamp', function () {
  it("select dateadd(ns,-1, to_timestamp_ntz('10000-01-01T00:00:00', 'YYYY-MM-DD\"T\"HH24:MI:SS')) AS C1;", function (done) {
    checkSingleTimestamp(
      '253402300799.999999999',
      'YYYY-MM-DD HH24:MI:SS.FF3',
      9,
      '9999-12-31 23:59:59.999',
      done,
      (actualTimestamp) => {
        assert.strictEqual(actualTimestamp.getNanoSeconds(), 999999999);
        assert.strictEqual(actualTimestamp.getEpochSeconds(), 253402300799);
        assert.strictEqual(actualTimestamp.getScale(), 9);
      },
    );
  });

  it("select to_timestamp_ntz('2024-04-16T14:57:58:999', 'YYYY-MM-DD\"T\"HH24:MI:SS:FF3') AS C1;", function (done) {
    checkSingleTimestamp(
      '1713279478.999',
      'YYYY-MM-DD HH24:MI:SS.FF3',
      3,
      '2024-04-16 14:57:58.999',
      done,
    );
  });

  it("select to_timestamp_ntz('2024-04-16T14:57:58:001', 'YYYY-MM-DD\"T\"HH24:MI:SS:FF3') AS C1;", function (done) {
    checkSingleTimestamp(
      '1713279478.001',
      'YYYY-MM-DD HH24:MI:SS.FF3',
      3,
      '2024-04-16 14:57:58.001',
      done,
    );
  });

  it("select to_timestamp_ntz('2024-04-16T14:57:58:999999999', 'YYYY-MM-DD\"T\"HH24:MI:SS:FF9') AS C1;", function (done) {
    checkSingleTimestamp(
      '1713279478.999999999',
      'YYYY-MM-DD HH24:MI:SS.FF9',
      9,
      '2024-04-16 14:57:58.999999999',
      done,
    );
  });

  it("select to_timestamp_ntz('2024-04-16T14:57:58:000000001', 'YYYY-MM-DD\"T\"HH24:MI:SS:FF9')) AS C1;", function (done) {
    checkSingleTimestamp(
      '1713279478.000000001',
      'YYYY-MM-DD HH24:MI:SS.FF9',
      9,
      '2024-04-16 14:57:58.000000001',
      done,
    );
  });
});
