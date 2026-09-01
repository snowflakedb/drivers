'use strict';

const { spawnSync } = require('child_process');
const path = require('path');

const { TestConfig } = require('./config');
const { createConnection, getServerVersion, executeSetupQueries, getDriverVersion } = require('./connection');
const { executeFetchTest } = require('./query_execution');
const { writeCsvResults, writeColdStartResults, writeMemoryTimeline, writeRunMetadata } = require('./results');

async function runColdStart(config) {
  // Each iteration is a fresh subprocess that imports, connects, and runs
  // SELECT 1 -- mirrors drivers/python/app/main.py's _run_cold_start exactly.
  const connectionParams = config.parseConnectionParams();
  const childEnv = {
    ...process.env,
    CONNECTION_PARAMS_JSON: JSON.stringify(connectionParams),
    DRIVER_TYPE: config.driverType,
  };
  const childScript = path.join(__dirname, 'cold_start_execution.js');

  console.log(`\n=== Cold-Start Test (${config.iterations} iterations) ===`);
  const rows = [];
  for (let i = 0; i < config.iterations; i++) {
    const label = `iter ${i + 1}/${config.iterations}`;
    const proc = spawnSync('node', [childScript], {
      cwd: __dirname,
      env: childEnv,
      encoding: 'utf8',
      timeout: 120000,
    });
    if (proc.status !== 0) {
      console.log(`  [${label}] FAILED (exit ${proc.status})`);
      console.error(proc.stderr);
      process.exit(1);
    }
    const line = proc.stdout.trim();
    console.log(`  [${label}] ${line}`);
    rows.push(line);
  }

  const filename = writeColdStartResults(rows, config.testName, config.driverType);
  writeRunMetadata(config.driverType, getDriverVersion(), 'N/A');

  console.log(`\n✓ Complete → ${filename}`);
}

async function main() {
  const config = new TestConfig();

  if (config.testType === 'cold_start') {
    await runColdStart(config);
    process.exit(0);
    return;
  }

  const connectionParams = config.parseConnectionParams();
  const setupQueries = config.getSetupQueries();

  let connection;
  try {
    connection = await createConnection(connectionParams);
  } catch (e) {
    console.error(`❌ Connection failed: ${e}`);
    process.exit(1);
  }

  try {
    await executeSetupQueries(connection, setupQueries);
  } catch (e) {
    console.error(`❌ Setup query failed: ${e}`);
    process.exit(1);
  }

  let results;
  let memoryTimeline;
  try {
    const outcome = await executeFetchTest(connection, config.sqlCommand, config.warmupIterations, config.iterations);
    results = outcome.results;
    memoryTimeline = outcome.memoryTimeline;
  } catch (e) {
    console.error(`❌ Test execution failed: ${e}`);
    process.exit(1);
  }

  const serverVersion = process.env.WIREMOCK_REPLAY === 'true' ? 'N/A' : await getServerVersion(connection);
  const driverVersion = getDriverVersion();
  writeRunMetadata(config.driverType, driverVersion, serverVersion || 'UNKNOWN');

  const filename = writeCsvResults(results, config.testName, config.driverType);
  const timelineFilename = writeMemoryTimeline(memoryTimeline, config.testName, config.driverType);

  console.log(`\n✓ Complete → ${filename}`);
  if (timelineFilename) {
    console.log(`✓ Memory timeline → ${timelineFilename}`);
  }

  process.exit(0);
}

main();
