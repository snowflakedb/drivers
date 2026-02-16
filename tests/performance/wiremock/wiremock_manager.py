"""WireMock container manager for HTTP traffic recording and replay"""
import json
import logging
import socket
from pathlib import Path
from typing import Optional

import requests
import time
import os
from testcontainers.core.container import DockerContainer

from .mapping_transformer import MappingTransformer

logger = logging.getLogger(__name__)

# Container name for WireMock (used for inter-container communication)
WIREMOCK_CONTAINER_NAME = "wiremock"


class WiremockManager:
    """
    Manage WireMock container lifecycle for recording and replaying HTTP traffic.
    
    Usage:
        # Recording phase
        wiremock = WiremockManager(mappings_dir)
        port = wiremock.start_recording()
        # ... run tests with proxy ...
        wiremock.create_snapshot()
        wiremock.stop()
        
        # Replay phase
        wiremock = WiremockManager(mappings_dir)
        port = wiremock.start_replay()
        # ... run tests with proxy ...
        wiremock.stop()
        wiremock.delete_mappings()
    """
    
    def __init__(
        self,
        mappings_dir: Path,
        image: str = "wiremock-perf:latest",
        network_mode: str = None
    ):
        """
        Initialize WireMock manager.
        
        Args:
            mappings_dir: Directory for storing/loading mappings
            image: Docker image name for WireMock
            network_mode: Docker network mode ("host" on Linux, None for default bridge on macOS)
        
        """
        self.mappings_dir = Path(mappings_dir)
        self.image = image
        self.network_mode = network_mode
        self.container: Optional[DockerContainer] = None
        self.port: Optional[int] = None
        self.host = "127.0.0.1"
        self.container_name = WIREMOCK_CONTAINER_NAME
        
    def start_recording(self) -> int:
        """
        Start WireMock in recording mode.
        
        Returns:
            Port number where WireMock is listening
        """
        # Find free port
        self.port = self._find_free_port()
        
        # Ensure mappings directory exists
        self.mappings_dir.mkdir(parents=True, exist_ok=True)
        
        # Build WireMock arguments
        wiremock_args = [
            "--port", str(self.port),
            "--root-dir", "/wiremock",
            "--enable-browser-proxying",
            "--preserve-host-header",
            "--https-keystore", "/certs/wiremock.jks",
            "--keystore-password", "password",
            "--ca-keystore", "/certs/wiremock.jks",
            "--ca-keystore-password", "password",
            "--disable-gzip",
            "--async-response-enabled",
            "--async-response-threads", "30",
            "--container-threads", "64"
        ]
        
        # Create and start container
        # For very large datasets (50M rows), use 12GB heap / 16GB container
        java_opts = "-Xmx12g -Xms4g -XX:+UseG1GC -XX:MaxGCPauseMillis=200"
        logger.info(f"Container memory limit: 16GB")
        
        # Get host user's UID:GID to run container as non-root
        # This ensures files are created with correct ownership
        user_spec = f"{os.getuid()}:{os.getgid()}"
        
        # Build kwargs dict - must set all at once (with_kwargs doesn't merge)
        container_kwargs = {
            "mem_limit": "16g",
            "memswap_limit": "16g",
            "user": user_spec  # Run as host user to avoid permission issues
        }
        if self.network_mode:
            container_kwargs["network_mode"] = self.network_mode
        
        container = (
            DockerContainer(self.image)
            .with_name(self.container_name)
            .with_command(" ".join(wiremock_args))
            .with_volume_mapping(str(self.mappings_dir), "/wiremock", mode="rw")
            .with_kwargs(**container_kwargs)
            .with_env("JAVA_OPTS", java_opts)
        )
        
        # Only bind ports if not using host network (host network uses ports directly)
        if self.network_mode != "host":
            container = container.with_bind_ports(self.port, self.port)
        
        self.container = container
        
        self.container.start()
        self._wait_for_wiremock()
        self._verify_jvm_config()

        # Add telemetry mock to avoid recording telemetry data
        self._add_telemetry_mock()
        
        network_info = " (host network)" if self.network_mode == "host" else " (bridge network)"
        logger.info(f"✓ WireMock recording on http://{self.host}:{self.port}{network_info}")
        return self.port
    
    def start_replay(self) -> int:
        """
        Start WireMock in replay mode (with disableRequestJournal for performance).
        
        Returns:
            Port number where WireMock is listening
        """
        self.port = self._find_free_port()
        
        # Verify mappings exist
        mappings_subdir = self.mappings_dir / "mappings"
        if not mappings_subdir.exists() or not list(mappings_subdir.glob("*.json")):
            raise RuntimeError(f"No mappings found in {mappings_subdir}")
        
        # Build WireMock arguments for optimal performance
        args = [
            "--port", str(self.port),
            "--enable-browser-proxying",
            "--https-keystore", "/certs/wiremock.jks",
            "--keystore-password", "password",
            "--ca-keystore", "/certs/wiremock.jks",
            "--ca-keystore-password", "password",
            "--root-dir", "/wiremock",
            "--proxy-pass-through", "false",  # disable pass-through in replay
            "--no-request-journal",  # Disable request history (saves memory)
            "--disable-request-logging",  # Disable per-request logging (5-20ms savings)
            "--disable-gzip",
            "--async-response-enabled",
            "--async-response-threads", "30",
            "--container-threads", "64",
            "--jetty-acceptor-threads", "4",  # Optimize connection acceptance
            "--jetty-accept-queue-size", "500"  # Handle connection bursts
        ]
        
        # Load custom extension for response time tracking
        args.extend(["--extensions", "extensions.ResponseTimeExtension"])
        
        # Create and start container
        # Optimize JVM for low latency: fixed heap size, aggressive GC tuning, minimal pause times
        java_opts = (
            "-Xmx12g -Xms12g "  # Fixed heap (avoids resize pauses)
            "-XX:+UseG1GC "  # G1 for low pause times
            "-XX:MaxGCPauseMillis=10 "  # Target 10ms max pause (was 200ms)
            "-XX:+ParallelRefProcEnabled "  # Parallel reference processing
            "-XX:InitiatingHeapOccupancyPercent=45 "  # Earlier GC to avoid spikes
            "-XX:G1ReservePercent=10"  # Reserve for to-space
        )
        logger.info(f"Configuring WireMock replay with low-latency JVM settings")
        logger.info(f"Container memory limit: 16GB")
        
        # Get host user's UID:GID to run container as non-root
        # This ensures files are created with correct ownership
        user_spec = f"{os.getuid()}:{os.getgid()}"
        
        container_kwargs = {
            "mem_limit": "16g",
            "memswap_limit": "16g",
            "user": user_spec  # Run as host user to avoid permission issues
        }
        if self.network_mode:
            container_kwargs["network_mode"] = self.network_mode
        
        container = (
            DockerContainer(self.image)
            .with_name(self.container_name)
            .with_command(" ".join(args))  # testcontainers expects a string
            .with_volume_mapping(str(self.mappings_dir), "/wiremock", mode="rw")  # Only mappings directory
            .with_kwargs(**container_kwargs)
            .with_env("JAVA_OPTS", java_opts)
        )
        
        # Only bind ports if not using host network (host network uses ports directly)
        if self.network_mode != "host":
            container = container.with_bind_ports(self.port, self.port)
        
        self.container = container
        
        self.container.start()
        logger.info("WireMock container started with memory configuration")
        
        try:
            self._wait_for_wiremock()
        except TimeoutError as e:
            # Capture container logs before failing
            logger.error("WireMock failed to start, capturing logs...")
            try:
                result = self.container.get_wrapped_container()
                logs = result.logs().decode('utf-8')
                logger.error("=== WireMock Container Logs (last 100 lines) ===")
                for line in logs.splitlines()[-100:]:
                    logger.error(line)
                logger.error("=== End Logs ===")
            except Exception as log_err:
                logger.error(f"Could not retrieve logs: {log_err}")
            raise e

        # Verify JVM configuration
        self._verify_jvm_config()

        # Note: Mappings are automatically loaded from /wiremock/mappings by WireMock
        # Verify mappings were loaded by checking files on disk
        mappings_path = self.mappings_dir / "mappings"
        mapping_files = list(mappings_path.glob("*.json"))
        logger.info(f"✓ {len(mapping_files)} mapping files available for replay")
        
        self._add_telemetry_mock()
        
        network_info = " (host network)" if self.network_mode == "host" else " (bridge network)"
        logger.info(f"✓ WireMock replay on http://{self.host}:{self.port}{network_info}")
        return self.port
    
    def create_snapshot(self):
        """
        Finalize recorded mappings and apply transformations.
        
        This method:
        1. Calls /recordings/snapshot with outputFormat=IDS and persist=True
        2. Verifies all mappings exist on disk
        3. Applies MappingTransformer to make them generic and reusable
        """
        logger.info("Creating snapshot to persist mappings (all bodies inline)...")
        response = requests.post(
            f"http://{self.host}:{self.port}/__admin/recordings/snapshot",
            json={
                "outputFormat": "IDS",  # Return only IDs, not full mappings
                "persist": True,  # Ensure mappings are written to disk
            },
            timeout=600
        )
        response.raise_for_status()
        
        # Verify mappings were saved to disk
        mappings_path = self.mappings_dir / "mappings"
        if not mappings_path.exists():
            raise RuntimeError(
                f"Mappings directory not found: {mappings_path}. "
                "WireMock failed to save recordings to disk."
            )
        
        mapping_files = list(mappings_path.glob("*.json"))
        if not mapping_files:
            raise RuntimeError(
                f"No mapping files found in {mappings_path}. "
                "WireMock recording may have failed."
            )
        
        logger.info(f"✓ Found {len(mapping_files)} mapping files on disk")
        
        # Apply transformations to make mappings generic
        # Note: WireMock runs as host user (not root), so no permission issues
        logger.info("Transforming mappings...")
        MappingTransformer.transform_mappings_directory(mappings_path)
    
    def stop(self):
        """Stop WireMock container"""
        if self.container:
            logger.info("Stopping WireMock...")
            self.container.stop()
            self.container = None
            self.port = None
            logger.info("✓ WireMock stopped")
    
    def delete_mappings(self):
        """Delete all mapping files"""
        if self.mappings_dir.exists():
            import shutil
            shutil.rmtree(self.mappings_dir)
            logger.info(f"✓ Deleted mappings: {self.mappings_dir}")
    
    def get_url(self) -> str:
        """Get WireMock proxy URL (for host access)"""
        if not self.port:
            raise RuntimeError("WireMock not started")
        return f"http://{self.host}:{self.port}"
    
    def get_container_name(self) -> str:
        """Get WireMock container name (for inter-container communication)"""
        return self.container_name

    @staticmethod
    def _find_free_port() -> int:
        """Find an available port"""
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            s.bind(('', 0))
            s.listen(1)
            port = s.getsockname()[1]
        return port
    
    def _wait_for_wiremock(self, timeout: int = 120):
        """
        Wait for WireMock to be ready using the health check endpoint.   
        """
        logger.info(f"Waiting for WireMock to start (timeout: {timeout}s)...")
        start_time = time.time()
        
        while time.time() - start_time < timeout:
            try:
                response = requests.get(
                    f"http://{self.host}:{self.port}/__admin/health",
                    timeout=2
                )
                if response.status_code == 200:
                    logger.info("✓ WireMock is ready")
                    return
            except requests.RequestException:
                # Connection not ready yet, wait and retry
                time.sleep(0.5)
        
        raise TimeoutError(f"WireMock did not start within {timeout}s")
    
    def _add_proxy_mapping(self, target_url: str, priority: int = 10):
        """Add catch-all proxy mapping"""
        # This creates a transparent proxy that forwards ALL requests
        # WireMock will preserve the Host header and proxy to the original destination
        mapping = {
            "request": {
                "method": "ANY",
                "urlPattern": ".*"
            },
            "response": {
                "proxyBaseUrl": target_url
            },
            "priority": priority,
            "persistent": False
        }
        
        response = requests.post(
            f"http://{self.host}:{self.port}/__admin/mappings",
            json=mapping
        )
        response.raise_for_status()
        logger.debug(f"Added catch-all proxy mapping for: {target_url}")
    
    def _add_telemetry_mock(self):
        """Add mock response for telemetry to avoid recording it"""
        mapping = {
            "request": {
                "method": "POST",
                "urlPathPattern": "/telemetry/send.*"
            },
            "response": {
                "status": 200,
                "headers": {
                    "Content-Type": "application/json"
                },
                "jsonBody": {
                    "data": "Log Received",
                    "code": None,
                    "message": None,
                    "success": True
                }
            },
            "priority": 1,  # Highest priority
            "persistent": False
        }
        
        response = requests.post(
            f"http://{self.host}:{self.port}/__admin/mappings",
            json=mapping
        )
        response.raise_for_status()
        logger.debug("Added telemetry mock")
    
    def _verify_jvm_config(self):
        """Verify JVM configuration was applied correctly"""
        try:
            wrapped = self.container.get_wrapped_container()
            # Get container logs to check JAVA_OPTS message
            logs = wrapped.logs().decode('utf-8')
            
            # Look for our JAVA_OPTS message in the logs
            for line in logs.splitlines():
                if "Started WireMock with JAVA_OPTS:" in line:
                    logger.info(f"✓ {line}")
                    return
            
            logger.warning("Could not verify JVM configuration from container logs")
        except Exception as e:
            logger.debug(f"Could not verify JVM config: {e}")
    
    def get_request_metrics(self) -> dict:
        """
        Retrieve request metrics from WireMock custom extension.
        
        Uses lightweight ResponseTimeExtension which tracks only timing data
        (not full request/response bodies). Reads stats from file written by extension.
        
        Returns:
            Dictionary containing response time statistics:
            - total_requests: Total number of requests processed
            - response_times: List of individual response times in milliseconds
            - metrics_enabled: Always True (metrics always collected)
        """
        # Read stats from file written by extension
        stats_file = self.mappings_dir / "response-time-stats.json"
        
        # Wait briefly for file to be created (extension writes after first request)
        max_wait = 5  # seconds
        wait_interval = 0.5
        waited = 0
        
        while not stats_file.exists() and waited < max_wait:
            time.sleep(wait_interval)
            waited += wait_interval
        
        empty_stats = {
            "total_requests": 0,
            "response_times": [],
            "metrics_enabled": True
        }
        
        try:
            if stats_file.exists():
                with open(stats_file, 'r') as f:
                    stats = json.load(f)
                stats["metrics_enabled"] = True
                return stats
            else:
                logger.warning(f"Stats file not found: {stats_file}")
                return empty_stats
            
        except Exception as e:
            logger.warning(f"Failed to read response time metrics from file: {e}")
            if stats_file.exists():
                try:
                    logger.warning(f"Stats file content (first 200 chars): {stats_file.read_text()[:200]}")
                except Exception:
                    pass  # Best-effort diagnostic — don't mask the original error
            return empty_stats

