"""In-process memory timeline monitor using /proc/self/statm.

Spawns a lightweight background thread that samples RSS and virtual memory
at a configurable interval (default 100ms). The thread reads /proc/self/statm
which is a single kernel-provided pseudo-file — no fork/exec overhead.

Linux only: /proc/self/statm is a Linux kernel interface. On other platforms
the monitor silently returns an empty timeline.

GIL note: both Event.wait() and file I/O release the GIL, so the monitor
thread does not contend with the main thread's CPU-bound work.
"""

import os
import sys
import threading
import time

IS_LINUX = sys.platform == "linux"


class MemorySample:
    __slots__ = ("timestamp_ms", "rss_bytes", "vm_bytes")

    def __init__(self, timestamp_ms: int, rss_bytes: int, vm_bytes: int):
        self.timestamp_ms = timestamp_ms
        self.rss_bytes = rss_bytes
        self.vm_bytes = vm_bytes


class ResourceMonitor:
    def __init__(self, interval_s: float = 0.1):
        self._interval = interval_s
        self._samples: list[MemorySample] = []
        self._stop = threading.Event()
        self._page_size = os.sysconf("SC_PAGE_SIZE") if IS_LINUX else 0
        self._thread: threading.Thread | None = None

    def _read_statm(self) -> tuple[int, int]:
        with open("/proc/self/statm") as f:
            parts = f.read().split()
        vm_pages = int(parts[0])
        rss_pages = int(parts[1])
        return rss_pages * self._page_size, vm_pages * self._page_size

    def start(self):
        if not IS_LINUX:
            return
        self._samples.clear()
        self._stop.clear()

        def _run():
            while not self._stop.is_set():
                rss, vm = self._read_statm()
                self._samples.append(
                    MemorySample(int(time.time() * 1000), rss, vm)
                )
                self._stop.wait(self._interval)

            rss, vm = self._read_statm()
            self._samples.append(
                MemorySample(int(time.time() * 1000), rss, vm)
            )

        self._thread = threading.Thread(
            target=_run, name="mem-monitor", daemon=True
        )
        self._thread.start()

    def stop(self) -> list[MemorySample]:
        if not IS_LINUX:
            return []
        self._stop.set()
        if self._thread is not None:
            self._thread.join(timeout=2)
        return list(self._samples)
