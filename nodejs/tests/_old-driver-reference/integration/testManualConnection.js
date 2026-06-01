const snowflake = require('./../../lib/snowflake').default;
const assert = require('assert');
const connOption = require('./connectionOptions');
const testUtil = require('./testUtil');
const Logger = require('../../lib/logger');

if (process.env.RUN_MANUAL_TESTS_ONLY === 'true') {
  describe('keepAlive test', function () {
    let connection;
    const loopCount = 10;
    const rowCount = 10;
    const tableName = 'test_keepalive000';

    const createTableWithRandomStrings = `CREATE OR REPLACE TABLE ${tableName} (value string)
    AS select randstr(200, random()) from table (generator(rowcount =>${rowCount}))`;

    before(async () => {
      connection = snowflake.createConnection(connOption.valid);
      await testUtil.connectAsync(connection);
      await testUtil.executeCmdAsync(connection, createTableWithRandomStrings);
    });
    after(async () => {
      snowflake.configure({ keepAlive: true });
      await testUtil.dropTablesIgnoringErrorsAsync(connection, [tableName]);
      await testUtil.destroyConnectionAsync(connection);
    });

    function executeSingleQuery() {
      return new Promise((resolve) => {
        const start = Date.now();
        connection.execute({
          sqlText: `SELECT VALUE
                      from ${tableName} limit ${rowCount};`,
          streamResult: true,
          complete: function (err, stmt) {
            if (err) {
              throw err;
            } else {
              stmt
                .streamRows()
                .on('error', function (err) {
                  throw err;
                })
                .on('end', function () {
                  const end = Date.now();
                  const time = end - start;
                  resolve(time);
                });
            }
          },
        });
      });
    }

    it('Verify that requests working faster with keep alive', async function () {
      let sumWithKeepAlive = 0;
      let sumWithoutKeepAlive = 0;
      for (let count = 1; count <= loopCount; count++) {
        const time = await executeSingleQuery();
        sumWithKeepAlive += time;
      }
      snowflake.configure({ keepAlive: false });
      for (let count = 1; count <= loopCount; count++) {
        const time = await executeSingleQuery();
        sumWithoutKeepAlive += time;
      }
      Logger.getInstance().info(
        `Sum of time without keep alive: ${sumWithoutKeepAlive}. Sum of time with keep alive:: ${sumWithKeepAlive}`,
      );
      assert.ok(
        sumWithoutKeepAlive * 0.66 > sumWithKeepAlive,
        'With keep alive the queries should work faster',
      );
    });
  });

  // Before run below tests you should prepare files connections.toml and token
  describe('Connection file configuration test', function () {
    afterEach(function () {
      delete process.env.SNOWFLAKE_HOME;
      delete process.env.SNOWFLAKE_DEFAULT_CONNECTION_NAME;
    });
    beforeEach(function () {
      snowflake.configure({ logLevel: 'DEBUG' });
    });

    it('test simple connection', async function () {
      await verifyConnectionWorks();
    });

    it('test connection with token', async function () {
      process.env.SNOWFLAKE_DEFAULT_CONNECTION_NAME = 'aws-oauth';
      await verifyConnectionWorks();
    });

    it('test connection with token using accessUrl', async function () {
      process.env.SNOWFLAKE_DEFAULT_CONNECTION_NAME = 'aws-oauth-accessUrl';
      await verifyConnectionWorks();
    });

    it('test connection with token from file', async function () {
      process.env.SNOWFLAKE_DEFAULT_CONNECTION_NAME = 'aws-oauth-file';
      await verifyConnectionWorks();
    });

    it('test pool simple connection', async function () {
      await verifyPoolConnectionWorks();
    });

    it('test pool connection with token', async function () {
      process.env.SNOWFLAKE_DEFAULT_CONNECTION_NAME = 'aws-oauth';
      await verifyPoolConnectionWorks();
    });

    it('test pool connection with token using accessUrl', async function () {
      process.env.SNOWFLAKE_DEFAULT_CONNECTION_NAME = 'aws-oauth-accessUrl';
      await verifyPoolConnectionWorks();
    });

    it('test pool connection with token from file', async function () {
      process.env.SNOWFLAKE_DEFAULT_CONNECTION_NAME = 'aws-oauth-file';
      await verifyPoolConnectionWorks();
    });

    async function verifyConnectionWorks(configuration) {
      const connection = snowflake.createConnection(configuration);
      await testUtil.connectAsync(connection);
      assert.ok(connection.isUp(), 'not active');
      await testUtil.executeCmdAsync(connection, 'Select 1');
      await testUtil.destroyConnectionAsync(connection);
    }
    async function verifyPoolConnectionWorks() {
      const connectionPool = snowflake.createPool(null, {
        max: 10,
        min: 2,
      });
      await connectionPool.use(async (clientConnection) => {
        return new Promise((resolve, reject) => {
          clientConnection.execute({
            sqlText: 'select 1;',
            complete: function (err) {
              if (err) {
                reject(err);
              }
              resolve();
            },
          });
        });
      });
    }
  });
}
