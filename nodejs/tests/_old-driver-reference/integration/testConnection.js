const snowflake = require('./../../lib/snowflake').default;
const assert = require('assert');
const connOption = require('./connectionOptions');
const testUtil = require('./testUtil');

// TODO: Parking heartbeat for now.
// When implementing in UD:
// - .heartbeat() + .heartbeatAsync() should be removed (not in public documentation).
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
