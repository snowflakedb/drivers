'use strict';

// Both driver stages install their package under the name `snowflake-sdk`
// (the old npm package in the "old" image, the local nodejs/ package in the
// "universal" image) -- so `require('snowflake-sdk')` resolves to whichever
// one is present, mirroring how the python driver just does
// `from snowflake import connector` regardless of driver_type.

function createConnection(connectionParams) {
  const sdk = require('snowflake-sdk');
  const connection = sdk.createConnection(connectionParams);
  return connection.connectAsync().then(() => connection);
}

function getServerVersion(connection) {
  return new Promise((resolve) => {
    connection.execute({
      sqlText: 'SELECT CURRENT_VERSION() AS VERSION',
      complete: (err, stmt, rows) => {
        if (err || !rows || !rows.length) {
          console.warn(`Warning: Could not retrieve server version: ${err}`);
          resolve('UNKNOWN');
          return;
        }
        const row = rows[0];
        const version = row.VERSION ?? Object.values(row)[0];
        resolve(version || 'UNKNOWN');
      },
    });
  });
}

function executeSetupQueries(connection, setupQueries) {
  if (!setupQueries || !setupQueries.length) {
    return Promise.resolve();
  }
  console.log(`\n=== Executing Setup Queries (${setupQueries.length} queries) ===`);
  return setupQueries
    .reduce(
      (chain, query, idx) =>
        chain.then(
          () =>
            new Promise((resolve, reject) => {
              console.log(`  Setup query ${idx + 1}: ${query}`);
              connection.execute({
                sqlText: query,
                complete: (err) => {
                  if (err) {
                    console.error(`\nERROR: Setup query ${idx + 1} failed: ${query}`);
                    console.error(`   Error: ${err}`);
                    reject(err);
                    return;
                  }
                  resolve();
                },
              });
            }),
        ),
      Promise.resolve(),
    )
    .then(() => console.log('Setup queries completed'));
}

function getDriverVersion() {
  try {
    // eslint-disable-next-line global-require
    return require('snowflake-sdk/package.json').version;
  } catch (e) {
    return 'UNKNOWN';
  }
}

module.exports = { createConnection, getServerVersion, executeSetupQueries, getDriverVersion };
