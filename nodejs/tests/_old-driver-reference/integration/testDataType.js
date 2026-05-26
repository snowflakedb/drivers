const async = require('async');
const assert = require('assert');
const GlobalConfig = require('./../../lib/global_config');
const snowflake = require('./../../lib/snowflake').default;
const testUtil = require('./testUtil');
const bigInt = require('big-integer');

describe('Test DataType', function () {
  let connection;
  const createTableWithVariant = 'create or replace table testVariant(colA variant)';
  const createTableWithNumber = 'create or replace table testNumber(colA number)';
  const createTableWithDouble = 'create or replace table testDouble(colA double)';
  const dropTableWithString = 'drop table if exists testString';
  const dropTableWithVariant = 'drop table if exists testVariant';
  const dropTableWithArray = 'drop table if exists testArray';
  const dropTableWithNumber = 'drop table if exists testNumber';
  const dropTableWithDouble = 'drop table if exists testDouble';
  const dropTableWithDate = 'drop table if exists testDate';
  const dropTableWithTime = 'drop table if exists testTime';
  const dropTableWithTimestamp = 'drop table if exists testTimestamp';
  const dropTableWithBoolean = 'drop table if exists testBoolean';
  const truncateTableWithVariant = 'truncate table if exists testVariant;';
  const insertDouble = 'insert into testDouble values(123.456)';
  const insertLargeNumber =
    'insert into testNumber values (12345678901234567890123456789012345678)';
  const insertRegularSizedNumber = 'insert into testNumber values (100000001)';
  const insertVariantJSON =
    "insert into testVariant select parse_json('{a : 1 , b :[1 , 2 , 3, -Infinity, undefined], c : {a : 1}}')";
  const insertVariantJSONForCustomParser =
    "insert into testVariant select parse_json('{a : 1 , b :[1 , 2 , 3], c : {a : 1}}')";
  const insertVariantXML =
    "insert into testVariant select parse_xml('<root><a>1</a><b>1</b><c><a>1</a></c></root>')";
  const selectDouble = 'select * from testDouble';
  const selectNumber = 'select * from testNumber';
  const selectVariant = 'select * from testVariant';

  before(function (done) {
    connection = testUtil.createConnection();
    async.series(
      [
        function (callback) {
          testUtil.connect(connection, callback);
        },
      ],
      done,
    );
  });

  after(function (done) {
    async.series(
      [
        function (callback) {
          testUtil.executeCmd(connection, dropTableWithString, callback);
        },
        function (callback) {
          testUtil.executeCmd(connection, dropTableWithVariant, callback);
        },
        function (callback) {
          testUtil.executeCmd(connection, dropTableWithArray, callback);
        },
        function (callback) {
          testUtil.executeCmd(connection, dropTableWithNumber, callback);
        },
        function (callback) {
          testUtil.executeCmd(connection, dropTableWithDouble, callback);
        },
        function (callback) {
          testUtil.executeCmd(connection, dropTableWithDate, callback);
        },
        function (callback) {
          testUtil.executeCmd(connection, dropTableWithTime, callback);
        },
        function (callback) {
          testUtil.executeCmd(connection, dropTableWithTimestamp, callback);
        },
        function (callback) {
          testUtil.executeCmd(connection, dropTableWithBoolean, callback);
        },
        function (callback) {
          testUtil.destroyConnection(connection, callback);
        },
      ],
      done,
    );
  });

  describe('testNumber', function () {
    it('testDouble', function (done) {
      async.series(
        [
          function (callback) {
            testUtil.executeCmd(connection, createTableWithDouble, callback);
          },
          function (callback) {
            testUtil.executeCmd(connection, insertDouble, callback);
          },
          function (callback) {
            testUtil.executeQueryAndVerify(connection, selectDouble, [{ COLA: 123.456 }], callback);
          },
        ],
        done,
      );
    });

    it('testLargeNumber', function (done) {
      async.series(
        [
          function (callback) {
            testUtil.executeCmd(connection, createTableWithNumber, callback);
          },
          function (callback) {
            testUtil.executeCmd(connection, insertLargeNumber, callback);
          },
          function (callback) {
            testUtil.executeQueryAndVerify(
              connection,
              selectNumber,
              [{ COLA: 1.2345678901234568e37 }],
              callback,
            );
          },
        ],
        done,
      );
    });

    it('testRegularSizedInteger', function (done) {
      async.series(
        [
          function (callback) {
            testUtil.executeCmd(connection, createTableWithNumber, callback);
          },
          function (callback) {
            testUtil.executeCmd(connection, insertRegularSizedNumber, callback);
          },
          function (callback) {
            testUtil.executeQueryAndVerify(
              connection,
              selectNumber,
              [{ COLA: 100000001 }],
              callback,
            );
          },
        ],
        done,
      );
    });
  });

  describe('testSemiStructuredDataType', function () {
    describe('testVariant', function () {
      before(async () => {
        await testUtil.executeCmdAsync(connection, createTableWithVariant);
      });

      after(async () => {
        await testUtil.executeCmdAsync(connection, dropTableWithVariant);
      });

      afterEach(async () => {
        await testUtil.executeCmdAsync(connection, truncateTableWithVariant);
      });

      it('testJSON', function (done) {
        async.series(
          [
            function (callback) {
              testUtil.executeCmd(connection, insertVariantJSON, callback);
            },
            function (callback) {
              testUtil.executeQueryAndVerify(
                connection,
                selectVariant,
                [{ COLA: { a: 1, b: [1, 2, 3, -Infinity, undefined], c: { a: 1 } } }],
                callback,
                null,
                true,
                false,
              );
            },
          ],
          done,
        );
      });

      it('testXML', function (done) {
        async.series(
          [
            function (callback) {
              testUtil.executeCmd(connection, insertVariantXML, callback);
            },
            function (callback) {
              testUtil.executeQueryAndVerify(
                connection,
                selectVariant,
                [{ COLA: { root: { a: 1, b: 1, c: { a: 1 } } } }],
                callback,
                null,
                true,
                false,
              );
            },
          ],
          done,
        );
      });

      describe('testCustomParser', function () {
        let originalParserConfig;

        before(() => {
          originalParserConfig = {
            jsonColumnVariantParser: GlobalConfig.jsonColumnVariantParser,
            xmlColumnVariantParser: GlobalConfig.xmlColumnVariantParser,
          };
        });

        after(() => {
          snowflake.configure(originalParserConfig);
        });

        it('testJSONCustomParser', function (done) {
          async.series(
            [
              function (callback) {
                snowflake.configure({
                  jsonColumnVariantParser: (rawColumnValue) => JSON.parse(rawColumnValue),
                });
                testUtil.executeCmd(connection, insertVariantJSONForCustomParser, callback);
              },
              function (callback) {
                testUtil.executeQueryAndVerify(
                  connection,
                  selectVariant,
                  [{ COLA: { a: 1, b: [1, 2, 3], c: { a: 1 } } }],
                  callback,
                );
              },
            ],
            done,
          );
        });
      });
    });
  });
});

describe('JS_TREAT_INTEGER_AS_BIGINT', () => {
  let connection;

  before(async () => {
    connection = testUtil.createConnection({
      jsTreatIntegerAsBigInt: true,
    });
    await testUtil.connectAsync(connection);
    await testUtil.executeCmdAsync(
      connection,
      'alter session set ENABLE_STRUCTURED_TYPES_IN_CLIENT_RESPONSE = true;',
    );
    await testUtil.executeCmdAsync(
      connection,
      'alter session set IGNORE_CLIENT_VESRION_IN_STRUCTURED_TYPES_RESPONSE = true;',
    );
  });

  function getFirstRowValue(rows) {
    return Object.values(rows[0])[0];
  }

  it('returns integer as BigInt', async () => {
    const { rows } = await testUtil.executeCmdAsync(connection, `select 4611693738694448603`);
    const selectedValue = getFirstRowValue(rows);
    assert.ok(bigInt.isInstance(selectedValue));
    assert.strictEqual(selectedValue.toString(), bigInt('4611693738694448603').toString());
  });

  // TODO: https://snowflakecomputing.atlassian.net/browse/SNOW-3155825
  // We need to revisit JSON/ARRAY column types and their conversion to JS objects.
  // Regardless of whether structured types are enabled or not, JSON.parse will lose precision.
  it('returns integer as number with precision loss in structured JSON', async () => {
    const { rows } = await testUtil.executeCmdAsync(
      connection,
      `select {'bigIntVal': 4611693738694448603}::OBJECT(bigIntVal BIGINT)`,
    );
    const selectedValue = getFirstRowValue(rows).bigIntVal;
    assert.strictEqual(Number.isInteger(selectedValue), true);
    assert.strictEqual(Number.isSafeInteger(selectedValue), false);
  });

  it('returns float as number with precision loss', async () => {
    const { rows } = await testUtil.executeCmdAsync(
      connection,
      'select 4611693738694448603.45::FLOAT',
    );
    const selectedValue = getFirstRowValue(rows);
    assert.strictEqual(Number.isInteger(selectedValue), true);
    assert.strictEqual(Number.isSafeInteger(selectedValue), false);
  });

  it('returns float as number with precision loss in structured JSON', async () => {
    const { rows } = await testUtil.executeCmdAsync(
      connection,
      `select {'floatVal': 4611693738694448603.45}::OBJECT(floatVal FLOAT)`,
    );
    const selectedValue = getFirstRowValue(rows).floatVal;
    assert.strictEqual(Number.isInteger(selectedValue), true);
    assert.strictEqual(Number.isSafeInteger(selectedValue), false);
  });
});
