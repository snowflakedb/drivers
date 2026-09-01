'use strict';

const { runWarmup, runTestIterations, printTimingStats, getPeakRssMb } = require('./common');
const { ResourceMonitor } = require('./resource_monitor');

async function executeFetchTest(connection, sqlCommand, warmupIterations, iterations) {
  console.log('\n=== Executing SELECT Test ===');
  console.log(`Query: ${sqlCommand}`);

  await runWarmup(_executeQuery, connection, sqlCommand, warmupIterations);

  const monitor = new ResourceMonitor(100);
  monitor.start();

  const results = await runTestIterations(_executeQuery, connection, sqlCommand, iterations);

  const memoryTimeline = monitor.stop();

  _validateRowCounts(results);
  _printStatistics(results);
  console.log(`  Memory timeline: ${memoryTimeline.length} samples collected`);

  return { results, memoryTimeline };
}

function _validateRowCounts(results) {
  if (!results.length) return;

  const expectedFromRecording = process.env.EXPECTED_ROW_COUNT;
  let expectedCount;
  let startIdx;
  if (expectedFromRecording) {
    expectedCount = parseInt(expectedFromRecording, 10);
    console.log(`Row count baseline: ${expectedCount} rows (from recording phase)`);
    startIdx = 0;
  } else {
    expectedCount = results[0].rowCount;
    console.log(`Row count baseline: ${expectedCount} rows (from first iteration)`);
    startIdx = 1;
  }

  if (expectedCount === 0) {
    throw new Error(
      'Row count baseline is 0 - this likely indicates a silent query failure. Refusing to use 0 as baseline.',
    );
  }

  for (let i = startIdx; i < results.length; i++) {
    if (results[i].rowCount !== expectedCount) {
      throw new Error(
        `Row count mismatch: iteration ${i} returned ${results[i].rowCount} rows, expected ${expectedCount} rows`,
      );
    }
  }
  console.log(`✓ All ${results.length} iterations returned ${expectedCount} rows`);
}

function _printStatistics(results) {
  console.log('\nSummary:');
  printTimingStats(
    'Fetch',
    results.map((r) => r.fetchTimeS),
  );
}

// NOTE: unlike python/odbc/jdbc, this does not split query_s vs fetch_s.
// Both old (snowflake-sdk) and the new nodejs/ package's public execute()
// API resolve once the full result is drained internally, with no exposed
// boundary between "query submitted" and "rows fetched" at that layer.
// This is intentional (see plan) -- it does not affect cpu_time_s/peak_rss_mb,
// which wrap the entire operation regardless.
function _executeQuery(connection, sql) {
  return new Promise((resolve, reject) => {
    const cpuStart = process.cpuUsage();
    const fetchStart = Date.now();

    connection.execute({
      sqlText: sql,
      complete: (err, stmt, rows) => {
        if (err) {
          reject(err);
          return;
        }
        const fetchTimeS = (Date.now() - fetchStart) / 1000;
        const cpuUsage = process.cpuUsage(cpuStart);
        const cpuTimeS = (cpuUsage.user + cpuUsage.system) / 1e6;
        const peakRssMb = getPeakRssMb();

        resolve({
          timestamp: Date.now(),
          fetchTimeS,
          rowCount: rows ? rows.length : 0,
          cpuTimeS,
          peakRssMb,
        });
      },
    });
  });
}

module.exports = { executeFetchTest };
