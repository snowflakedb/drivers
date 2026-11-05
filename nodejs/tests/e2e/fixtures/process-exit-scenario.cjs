// Usage: node process-exit-scenario.cjs <resource> <lifecycle> <connectionOptionsJson>
// Never calls `process.exit`, so it only exits once the driver releases every handle.
const http = require('node:http');
const snowflake = require('snowflake-sdk');

const [resource, lifecycle, connectionOptionsJson] = process.argv.slice(2);

const connect = (options = {}) =>
  snowflake.createConnection({
    ...JSON.parse(connectionOptionsJson),
    account: 'testaccount',
    username: 'alice',
    password: 'test-password',
    ...options,
  });

const connectWithExternalBrowser = (options = {}) =>
  connect({
    authenticator: 'EXTERNALBROWSER',
    openExternalBrowserCallback: (url) => {
      const port = new URL(url).searchParams.get('browser_mode_redirect_port');
      http.get(`http://127.0.0.1:${port}/?token=test-token`, (res) => res.resume());
    },
    ...options,
  });

const RESOURCES = {
  'no resource-holding options': () => connect(),
  openExternalBrowserCallback: () => connectWithExternalBrowser(),
  browserResponseRenderer: () =>
    connectWithExternalBrowser({ browserResponseRenderer: () => 'done' }),
  customCredentialManager: () => {
    snowflake.configure({
      customCredentialManager: { read: () => null, write: () => {}, remove: () => {} },
    });
    return connect();
  },
  customLogger: () => {
    const log = () => {};
    snowflake.configure({
      logLevel: 'DEBUG',
      customLogger: { error: log, warn: log, info: log, debug: log, trace: log },
    });
    return connect();
  },
  clientSessionKeepAlive: () => connect({ clientSessionKeepAlive: true }),
};

const LIFECYCLES = {
  'never connected': () => {},
  'connected then destroyed': (connection) =>
    connection.connectAsync().then(
      () => connection.destroy(() => {}),
      (err) => {
        console.error(err);
        process.exitCode = 1;
      },
    ),
  'failed to connect': (connection) => connection.connectAsync().catch(() => {}),
};

module.exports = { RESOURCE_NAMES: Object.keys(RESOURCES) };

if (require.main === module) {
  LIFECYCLES[lifecycle](RESOURCES[resource]());
}
