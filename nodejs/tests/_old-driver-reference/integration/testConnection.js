const snowflake = require('./../../lib/snowflake').default;
const assert = require('assert');
const connOption = require('./connectionOptions');
const testUtil = require('./testUtil');

// TODO: Parking heartbeat + isValid for now.
// When implementing in UD:
// - .heartbeat() + .heartbeatAsync() should be removed (not in public documentation).
// - .isValidAsync() - must ensure that sf_core properly implements it.
describe('Connection Test - Heartbeat', () => {
  let connection;

  before(async () => {
    connection = snowflake.createConnection(connOption.valid);
    await testUtil.connectAsync(connection);
  });

  after(async () => {
    await testUtil.destroyConnectionAsync(connection);
  });

  it('call heartbeat url with default callback', () => {
    connection.heartbeat();
  });

  it('call heartbeat url with callback', (done) => {
    connection.heartbeat((err) => (err ? done(err) : done()));
  });

  it('call heartbeat url as promise', async () => {
    const rows = await connection.heartbeatAsync();
    assert.deepEqual(rows, [{ 1: 1 }]);
  });
});

describe('Connection Test - isValid', () => {
  let connection;

  beforeEach(async () => {
    connection = snowflake.createConnection(connOption.valid);
    await testUtil.connectAsync(connection);
  });

  afterEach(async () => {
    if (connection.isUp()) {
      await testUtil.destroyConnectionAsync(connection);
    }
  });

  it('connection is valid after connect', async () => {
    const result = await connection.isValidAsync();

    assert.equal(result, true);
  });

  it('connection is invalid after destroy', async () => {
    await testUtil.destroyConnectionAsync(connection);

    const result = await connection.isValidAsync();

    assert.equal(result, false);
  });

  // there is no way to test heartbeat fail to running instance of snowflake
});
