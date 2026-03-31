"""Background monitor for WireMock container CPU and memory usage.

Uses the Docker stats streaming API (same data as ``docker stats``) to
sample cgroup-level CPU and memory at ~1 s intervals.  On stop it also
captures the full container log output.
"""

import logging
import threading
import time
from dataclasses import dataclass, field
from typing import Optional

logger = logging.getLogger(__name__)


@dataclass
class ContainerSample:
    timestamp_ms: int
    cpu_percent: float
    memory_usage_mb: float
    memory_limit_mb: float


@dataclass
class MonitorResult:
    samples: list[ContainerSample] = field(default_factory=list)
    logs: str = ""


class WiremockMonitor:
    """Poll Docker container stats in a background thread."""

    def __init__(self, docker_container, interval_s: float = 1.0):
        """
        Args:
            docker_container: A running Docker container object (from the
                Docker SDK, i.e. ``container.get_wrapped_container()``).
            interval_s: Approximate polling interval in seconds.
        """
        self._container = docker_container
        self._interval = interval_s
        self._stop = threading.Event()
        self._thread: Optional[threading.Thread] = None
        self._samples: list[ContainerSample] = []

    def start(self):
        self._stop.clear()
        self._samples.clear()
        self._thread = threading.Thread(
            target=self._poll, name="wiremock-monitor", daemon=True
        )
        self._thread.start()

    def stop(self) -> MonitorResult:
        self._stop.set()
        if self._thread is not None:
            self._thread.join(timeout=5)

        logs = self._capture_logs()
        return MonitorResult(samples=list(self._samples), logs=logs)

    def _poll(self):
        prev_cpu: Optional[int] = None
        prev_system: Optional[int] = None

        while not self._stop.is_set():
            try:
                stats = self._container.stats(stream=False)
                sample = self._parse_stats(stats, prev_cpu, prev_system)

                if sample is not None:
                    self._samples.append(sample)

                cpu_stats = stats.get("cpu_stats", {})
                prev_cpu = cpu_stats.get("cpu_usage", {}).get("total_usage", 0)
                prev_system = cpu_stats.get("system_cpu_usage", 0)

            except Exception as e:
                logger.debug(f"WiremockMonitor poll error: {e}")

            self._stop.wait(self._interval)

    @staticmethod
    def _parse_stats(
        stats: dict,
        prev_cpu: Optional[int],
        prev_system: Optional[int],
    ) -> Optional[ContainerSample]:
        cpu_stats = stats.get("cpu_stats", {})
        precpu_stats = stats.get("precpu_stats", {})
        memory_stats = stats.get("memory_stats", {})

        memory_usage = memory_stats.get("usage", 0)
        memory_limit = memory_stats.get("limit", 0)

        cpu_total = cpu_stats.get("cpu_usage", {}).get("total_usage", 0)
        system_cpu = cpu_stats.get("system_cpu_usage", 0)

        if prev_cpu is not None and prev_system is not None:
            cpu_delta = cpu_total - prev_cpu
            system_delta = system_cpu - prev_system
        else:
            pre_cpu_total = precpu_stats.get("cpu_usage", {}).get("total_usage", 0)
            pre_system = precpu_stats.get("system_cpu_usage", 0)
            cpu_delta = cpu_total - pre_cpu_total
            system_delta = system_cpu - pre_system

        num_cpus = len(cpu_stats.get("cpu_usage", {}).get("percpu_usage", [])) or 1
        if system_delta > 0 and cpu_delta >= 0:
            cpu_percent = (cpu_delta / system_delta) * num_cpus * 100.0
        else:
            cpu_percent = 0.0

        return ContainerSample(
            timestamp_ms=int(time.time() * 1000),
            cpu_percent=round(cpu_percent, 2),
            memory_usage_mb=round(memory_usage / (1024 * 1024), 1),
            memory_limit_mb=round(memory_limit / (1024 * 1024), 1),
        )

    def _capture_logs(self) -> str:
        try:
            return self._container.logs().decode("utf-8", errors="replace")
        except Exception as e:
            logger.warning(f"Failed to capture WireMock logs: {e}")
            return ""
