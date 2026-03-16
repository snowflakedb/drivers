"""Proxy server container manager for HTTP traffic recording and replay."""

import json
import logging
import os
import shutil
import socket
import time
from pathlib import Path

import requests
from testcontainers.core.container import DockerContainer

logger = logging.getLogger(__name__)

CONTAINER_NAME = "proxy-server"


class ProxyServerManager:
    """
    Manage the proxy server container lifecycle for recording and replay.

    Recording mode:
        start_recording, create_snapshot, stop, get_url,
        export_ca_cert

    Replay mode:
        start_replay, stop, get_url, get_container_name,
        export_ca_cert, flush_stats, get_request_metrics
    """

    def __init__(
        self,
        mappings_dir: Path,
        image: str = "replay-server:latest",
        network_mode: str = None,
    ):
        self.mappings_dir = Path(mappings_dir)
        self.image = image
        self.network_mode = network_mode
        self.container: DockerContainer | None = None
        self.port: int | None = None
        self.host = "127.0.0.1"
        self.container_name = CONTAINER_NAME
        self.driver_label: str | None = None

    # ── Recording Lifecycle ──────────────────────────────────────────

    def start_recording(self) -> int:
        """Start the proxy server in record mode (forwards to real backend)."""
        self.port = self._find_free_port()

        self.mappings_dir.mkdir(parents=True, exist_ok=True)

        args = [
            "--mappings-dir", "/data",
            "--port", str(self.port),
            "--mode", "record",
        ]

        return self._start_container(args, mem_limit="2g")

    def create_snapshot(self):
        """Transform raw recorded mappings for replay (runs on host)."""
        from replay_server.mapping_transformer import MappingTransformer

        mappings_path = self.mappings_dir / "mappings"
        if not mappings_path.exists():
            raise RuntimeError(
                f"Mappings directory not found: {mappings_path}. "
                "Recording may have failed."
            )

        mapping_files = list(mappings_path.glob("*.json"))
        if not mapping_files:
            raise RuntimeError(
                f"No mapping files found in {mappings_path}. "
                "Recording may have failed."
            )

        logger.info(f"✓ Found {len(mapping_files)} mapping files on disk")
        logger.info("Transforming mappings...")
        MappingTransformer.transform_mappings_directory(mappings_path)

    # ── Replay Lifecycle ─────────────────────────────────────────────

    def start_replay(self, driver_label: str = None) -> int:
        """Start the proxy server in replay mode (serves from disk)."""
        self.port = self._find_free_port()
        self.driver_label = driver_label

        mappings_subdir = self.mappings_dir / "mappings"
        if not mappings_subdir.exists() or not list(mappings_subdir.glob("*.json")):
            raise RuntimeError(f"No mappings found in {mappings_subdir}")

        args = [
            "--mappings-dir", "/data",
            "--port", str(self.port),
            "--mode", "replay",
        ]
        if driver_label:
            args.extend(["--stats-suffix", driver_label])

        port = self._start_container(args, mem_limit="4g")

        mapping_files = list(mappings_subdir.glob("*.json"))
        logger.info(f"✓ {len(mapping_files)} mapping files loaded by replay server")
        return port

    # ── Common Lifecycle ─────────────────────────────────────────────

    def stop(self):
        """Stop the container."""
        if self.container:
            logger.info("Stopping proxy server...")
            self.container.stop()
            self.container = None
            self.port = None
            logger.info("✓ Proxy server stopped")

    def _start_container(self, args: list[str], mem_limit: str = "2g") -> int:
        """Start the Docker container with the given arguments."""
        user_spec = f"{os.getuid()}:{os.getgid()}"
        container_kwargs = {
            "mem_limit": mem_limit,
            "memswap_limit": mem_limit,
            "user": user_spec,
        }
        if self.network_mode:
            container_kwargs["network_mode"] = self.network_mode

        container = (
            DockerContainer(self.image)
            .with_name(self.container_name)
            .with_command(" ".join(args))
            .with_volume_mapping(str(self.mappings_dir), "/data", mode="rw")
            .with_kwargs(**container_kwargs)
        )

        if self.network_mode != "host":
            container = container.with_bind_ports(self.port, self.port)

        self.container = container
        self.container.start()
        self._wait_for_ready()

        network_info = (" (host network)" if self.network_mode == "host"
                        else " (bridge network)")
        logger.info(
            f"✓ Proxy server on http://{self.host}:{self.port}{network_info}")
        return self.port

    # ── Interface Methods ────────────────────────────────────────────

    def get_url(self) -> str:
        if not self.port:
            raise RuntimeError("Proxy server not started")
        return f"http://{self.host}:{self.port}"

    def get_container_name(self) -> str:
        return self.container_name

    def export_ca_cert(self, target_dir: Path) -> Path:
        """Copy the CA cert generated by the server to target_dir."""
        ca_cert_src = self.mappings_dir / "replay-ca.crt"
        target_path = Path(target_dir) / "proxy-ca.crt"

        deadline = time.time() + 30
        while time.time() < deadline:
            if ca_cert_src.exists() and ca_cert_src.stat().st_size > 0:
                shutil.copy2(ca_cert_src, target_path)
                logger.debug(f"Exported CA cert -> {target_path}")
                return target_path
            time.sleep(0.2)

        raise RuntimeError(
            f"CA cert not found at {ca_cert_src} within 30s")

    def flush_stats(self):
        """Trigger the server to write response-time stats to disk."""
        if not self.port:
            logger.warning("Cannot flush stats — server not running")
            return
        try:
            requests.get(
                f"http://{self.host}:{self.port}/__perf/flush-stats", timeout=10)
            time.sleep(0.3)
        except requests.RequestException as e:
            logger.warning(f"Stats flush request failed: {e}")

    def get_request_metrics(self) -> dict:
        """Read response-time metrics from the stats JSON file on the volume."""
        self.flush_stats()

        suffix = f"-{self.driver_label}" if self.driver_label else ""
        stats_file = self.mappings_dir / f"response-time-stats{suffix}.json"

        if not stats_file.exists():
            logger.warning(f"Stats file not found: {stats_file}")
            return {"total_requests": 0, "response_times": [],
                    "metrics_enabled": True}

        try:
            with open(stats_file) as f:
                stats = json.load(f)
            stats["metrics_enabled"] = True
            return stats
        except Exception as e:
            logger.warning(f"Failed to read stats: {e}")
            return {"total_requests": 0, "response_times": [],
                    "metrics_enabled": True}

    # ── Internal ──────────────────────────────────────────────────────

    def _wait_for_ready(self, timeout: int = 60):
        """Poll the health endpoint until the server is ready."""
        logger.info(
            f"Waiting for proxy server to start (timeout: {timeout}s)...")
        deadline = time.time() + timeout

        while time.time() < deadline:
            try:
                resp = requests.get(
                    f"http://{self.host}:{self.port}/__admin/health", timeout=2)
                if resp.status_code == 200:
                    logger.info("✓ Proxy server is ready")
                    return
            except requests.RequestException:
                time.sleep(0.3)

        self._dump_container_logs()
        raise TimeoutError(
            f"Proxy server did not start within {timeout}s")

    def _dump_container_logs(self):
        """Print container logs on failure for debugging."""
        if not self.container:
            return
        try:
            wrapped = self.container.get_wrapped_container()
            logs = wrapped.logs().decode('utf-8')
            logger.error("=== Proxy Server Logs (last 50 lines) ===")
            for line in logs.splitlines()[-50:]:
                logger.error(line)
            logger.error("=== End Logs ===")
        except Exception as e:
            logger.error(f"Could not retrieve logs: {e}")

    @staticmethod
    def _find_free_port() -> int:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            s.bind(('', 0))
            s.listen(1)
            return s.getsockname()[1]
