const async = require('async');
const assert = require('assert');
const GlobalConfig = require('./../../lib/global_config');
const snowflake = require('./../../lib/snowflake').default;
const testUtil = require('./testUtil');

describe('Test DataType', function () {
  const createTableWithVariant = 'create or replace table testVariant(colA variant)';
  const dropTableWithVariant = 'drop table if exists testVariant';
  const truncateTableWithVariant = 'truncate table if exists testVariant;';
  const insertVariantJSON =
    "insert into testVariant select parse_json('{a : 1 , b :[1 , 2 , 3, -Infinity, undefined], c : {a : 1}}')";
  const insertVariantJSONForCustomParser =
    "insert into testVariant select parse_json('{a : 1 , b :[1 , 2 , 3], c : {a : 1}}')";
  const insertVariantXML =
    "insert into testVariant select parse_xml('<root><a>1</a><b>1</b><c><a>1</a></c></root>')";
  const selectVariant = 'select * from testVariant';

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
