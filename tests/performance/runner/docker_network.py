"""Docker network management for performance tests

Performance Optimization: Optimized Docker Networking
-----------------------------------------------------
This module uses platform-specific optimizations to reduce Docker networking jitter:
- Linux: host network mode (zero network overhead)
- macOS: default bridge with host.docker.internal (Docker Desktop limitation)

On macOS, Docker runs in a VM, so "--network host" doesn't expose ports to the actual
host. Instead, we use default bridge networking which works consistently everywhere.
"""
import logging
from contextlib import contextmanager
import platform

logger = logging.getLogger(__name__)

# Detect platform
IS_LINUX = platform.system() == "Linux"


class DockerNetworkManager:
    """
    Manage Docker network configuration for performance tests.
    
    Uses host network on Linux, default bridge on macOS for optimal performance.
    """
    
    def __init__(self, network_name: str = None):
        """Initialize network manager."""
        self.network = None
    
    def create_network(self) -> str:
        """
        Configure network mode for containers.
        
        Returns:
            "host" on Linux, None on macOS (default bridge)
        """
        if IS_LINUX:
            logger.info("✓ Using host network mode (zero overhead on Linux)")
            self.network = "host"
            return "host"
        else:
            logger.info("✓ Using default bridge network with host.docker.internal (macOS)")
            self.network = None  # Use default bridge
            return None
    
    def remove_network(self):
        """No-op - neither host nor default bridge need cleanup."""
        pass
    
    def get_network(self):
        """Get the current network mode."""
        return self.network


@contextmanager
def perf_network():
    """
    Context manager for configuring network mode for performance tests.
    
    Usage:
        with perf_network() as network_mode:
            # network_mode is "host"
            pass
    """
    manager = DockerNetworkManager()
    try:
        network_mode = manager.create_network()
        yield network_mode
    finally:
        manager.remove_network()
