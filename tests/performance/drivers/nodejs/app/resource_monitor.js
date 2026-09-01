'use strict';

const fs = require('fs');

// In-process memory timeline monitor using /proc/self/statm, mirroring the
// python/odbc/jdbc drivers' resource_monitor implementations exactly (same
// source, same 100ms default interval, same Linux-only constraint).
const IS_LINUX = process.platform === 'linux';

// Page size for the rss/vm page-count -> bytes conversion. Assumed 4096
// (standard on x86_64/arm64 glibc and musl Linux containers, which is what
// these images build for) -- unlike python's os.sysconf('SC_PAGE_SIZE'),
// Node has no built-in syscall for this, so this is a fixed assumption
// rather than a runtime query. Flag if targeting an unusual page size.
const PAGE_SIZE = 4096;

class ResourceMonitor {
  constructor(intervalMs = 100) {
    this.intervalMs = intervalMs;
    this.samples = [];
    this.timer = null;
  }

  _readStatm() {
    const parts = fs.readFileSync('/proc/self/statm', 'utf8').trim().split(/\s+/);
    const vmPages = parseInt(parts[0], 10);
    const rssPages = parseInt(parts[1], 10);
    return { rssBytes: rssPages * PAGE_SIZE, vmBytes: vmPages * PAGE_SIZE };
  }

  start() {
    if (!IS_LINUX) return;
    this.samples = [];
    const sample = () => {
      const { rssBytes, vmBytes } = this._readStatm();
      this.samples.push({ timestampMs: Date.now(), rssBytes, vmBytes });
    };
    sample();
    this.timer = setInterval(sample, this.intervalMs);
  }

  stop() {
    if (!IS_LINUX) return [];
    if (this.timer) {
      clearInterval(this.timer);
      this.timer = null;
    }
    const { rssBytes, vmBytes } = this._readStatm();
    this.samples.push({ timestampMs: Date.now(), rssBytes, vmBytes });
    return this.samples;
  }
}

module.exports = { ResourceMonitor };
