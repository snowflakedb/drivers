'use strict';

const fs = require('fs');
const path = require('path');

function _resultsDir(driverType, testName) {
  const subdir = testName.endsWith('_record') ? '_record' : testName;
  const dir = path.join('/results', driverType, subdir);
  fs.mkdirSync(dir, { recursive: true });
  return dir;
}

function writeCsvResults(results, testName, driverType) {
  const timestamp = Math.floor(Date.now() / 1000);
  const dir = _resultsDir(driverType, testName);
  const filename = path.join(dir, `${testName}_nodejs_${driverType}_${timestamp}.csv`);

  const header = 'timestamp_ms,fetch_s,row_count,cpu_time_s,peak_rss_mb\n';
  const rows = results.map(
    (r) => `${r.timestamp},${r.fetchTimeS.toFixed(6)},${r.rowCount},${r.cpuTimeS.toFixed(6)},${r.peakRssMb.toFixed(1)}`,
  );
  fs.writeFileSync(filename, header + rows.join('\n') + '\n');
  return filename;
}

function writeColdStartResults(rows, testName, driverType) {
  const timestamp = Math.floor(Date.now() / 1000);
  const dir = _resultsDir(driverType, testName);
  const filename = path.join(dir, `${testName}_nodejs_${driverType}_${timestamp}.csv`);

  const header = 'timestamp_ms,e2e_s,load_s,connect_s,select1_s,cpu_time_s,peak_rss_mb\n';
  fs.writeFileSync(filename, header + rows.join('\n') + '\n');
  return filename;
}

function writeMemoryTimeline(memoryTimeline, testName, driverType) {
  if (!memoryTimeline || !memoryTimeline.length) return null;

  const timestamp = Math.floor(Date.now() / 1000);
  const dir = _resultsDir(driverType, testName);
  const filename = path.join(dir, `memory_timeline_${testName}_nodejs_${driverType}_${timestamp}.csv`);

  const header = 'timestamp_ms,rss_bytes,vm_bytes\n';
  const rows = memoryTimeline.map((s) => `${s.timestampMs},${s.rssBytes},${s.vmBytes}`);
  fs.writeFileSync(filename, header + rows.join('\n') + '\n');
  return filename;
}

function writeRunMetadata(driverType, driverVersion, serverVersion) {
  const metadataFilename = path.join('/results', `run_metadata_nodejs_${driverType}.json`);
  if (fs.existsSync(metadataFilename)) return;

  const metadata = {
    driver: 'nodejs',
    driver_type: driverType,
    driver_version: driverVersion,
    runtime_language_version: process.version,
    server_version: serverVersion,
    architecture: _getArchitecture(),
    os: process.env.OS_INFO || 'Linux',
    run_timestamp: Math.floor(Date.now() / 1000),
  };

  if (driverType === 'universal') {
    metadata.build_rust_version = process.env.BUILD_RUST_VERSION || 'NA';
  }

  fs.writeFileSync(metadataFilename, JSON.stringify(metadata, null, 2));
}

function _getArchitecture() {
  const arch = process.arch; // 'x64', 'arm64', etc.
  if (arch === 'x64') return 'x86_64';
  if (arch === 'arm64') return 'arm64';
  return arch;
}

module.exports = { writeCsvResults, writeColdStartResults, writeMemoryTimeline, writeRunMetadata };
