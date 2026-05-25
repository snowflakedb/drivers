const assert = require('assert');
const testUtil = require('./testUtil');

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
