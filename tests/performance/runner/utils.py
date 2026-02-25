"""Utility functions for performance tests"""
import subprocess
from pathlib import Path


def repo_root() -> Path:
    """
    Get the repository root using git.
    
    Returns:
        Path to the repository root
        
    Raises:
        RuntimeError: If git command fails or repository root cannot be determined
    """
    result = subprocess.run(
        ["git", "rev-parse", "--show-toplevel"],
        capture_output=True,
        text=True,
    )
    if result.returncode == 0:
        root = result.stdout.strip()
        if root:
            return Path(root)
    raise RuntimeError("Failed to determine repository root")


def perf_tests_root() -> Path:
    """Get the performance tests directory root."""
    return repo_root() / "tests" / "performance"
