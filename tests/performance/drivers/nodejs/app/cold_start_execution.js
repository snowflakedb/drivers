'use strict';

// Cold-start child process: measures import -> connect -> SELECT 1.
//
// Invoked as a subprocess by main.js for each cold-start iteration. Prints a
// single CSV row to stdout so the parent can aggregate.
//
// Environment variables:
//   CONNECTION_PARAMS_JSON  - JSON-encoded connection options
//   DRIVER_TYPE             - "universal" or "old" (unused here: WireMock/
//                              recorded_http is not supported for nodejs yet,
//                              so there is no proxy/TLS relaxation to apply)

const t0 = process.hrtime.bigint();

const sdk = require('snowflake-sdk'); // import timing is the point

const t1 = process.hrtime.bigint();

function toSeconds(startNs, endNs) {
  return Number(endNs - startNs) / 1e9;
}

function connectAsync(params) {
  const connection = sdk.createConnection(params);
  return connection.connectAsync().then(() => connection);
}

function executeAsync(connection, sqlText) {
  return new Promise((resolve, reject) => {
    connection.execute({
      sqlText,
      complete: (err, stmt, rows) => {
        if (err) reject(err);
        else resolve(rows);
      },
    });
  });
}

async function main() {
  const connectionParams = JSON.parse(process.env.CONNECTION_PARAMS_JSON);

  const connection = await connectAsync(connectionParams);
  const t2 = process.hrtime.bigint();

  const rows = await executeAsync(connection, 'SELECT 1');
  const value = rows && rows.length > 0 ? Object.values(rows[0])[0] : undefined;
  if (Number(value) !== 1) {
    throw new Error(`Expected 1, got ${value}`);
  }

  const t3 = process.hrtime.bigint();

  const timestampMs = Date.now();
  const e2eS = toSeconds(t0, t3);
  const loadS = toSeconds(t0, t1);
  const connectS = toSeconds(t1, t2);
  const select1S = toSeconds(t2, t3);
  const cpuUsage = process.cpuUsage();
  const cpuTimeS = (cpuUsage.user + cpuUsage.system) / 1e6;
  const peakRssMb = process.resourceUsage().maxRSS / 1024;

  process.stdout.write(
    `${timestampMs},${e2eS.toFixed(6)},${loadS.toFixed(6)},${connectS.toFixed(6)},${select1S.toFixed(6)},${cpuTimeS.toFixed(6)},${peakRssMb.toFixed(1)}\n`,
  );
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
