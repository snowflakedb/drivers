'use strict';

function getPeakRssMb() {
  // Node documents resourceUsage().maxRSS as normalized to kilobytes on all
  // platforms (unlike raw POSIX getrusage, which is KB on Linux but bytes on
  // macOS -- exactly why python/odbc/jdbc each special-case units). Verify
  // this empirically in-container before trusting it on a new platform.
  const maxRssKb = process.resourceUsage().maxRSS;
  return maxRssKb / 1024;
}

async function runWarmup(executeFn, connection, sql, warmupIterations) {
  for (let i = 0; i < warmupIterations; i++) {
    await executeFn(connection, sql);
  }
}

async function runTestIterations(executeFn, connection, sql, iterations) {
  const results = [];
  for (let i = 0; i < iterations; i++) {
    results.push(await executeFn(connection, sql));
  }
  return results;
}

function median(values) {
  if (!values.length) return 0;
  const sorted = [...values].sort((a, b) => a - b);
  const mid = Math.floor(sorted.length / 2);
  return sorted.length % 2 !== 0 ? sorted[mid] : (sorted[mid - 1] + sorted[mid]) / 2;
}

function printTimingStats(label, values) {
  if (!values.length) {
    console.log(`  ${label}: no data`);
    return;
  }
  const med = median(values);
  const min = Math.min(...values);
  const max = Math.max(...values);
  console.log(`  ${label}: median=${med.toFixed(3)}s  min=${min.toFixed(3)}s  max=${max.toFixed(3)}s`);
}

module.exports = { getPeakRssMb, runWarmup, runTestIterations, printTimingStats, median };
