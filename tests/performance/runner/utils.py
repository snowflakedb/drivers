"""Utility functions for performance tests"""
import logging
import os
import re
import subprocess
from pathlib import Path

logger = logging.getLogger(__name__)


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


def _run_cmd(cmd: str) -> str:
    """Run a shell command and return stripped stdout, or empty string on failure."""
    try:
        result = subprocess.run(
            cmd, shell=True, capture_output=True, text=True, timeout=10,
        )
        return result.stdout.strip() if result.returncode == 0 else ""
    except Exception:
        return ""


def collect_node_info() -> dict:
    """
    Collect hardware and OS characteristics of the current host.

    Returns a dict suitable for use as Benchstore tags. All values are strings.
    On any failure a field falls back to 'UNKNOWN'.
    """
    info: dict[str, str] = {}

    # ── CPU model ──
    cpu_model = "UNKNOWN"
    cpu_info_path = Path("/proc/cpuinfo")
    try:
        if cpu_info_path.exists():
            raw = cpu_info_path.read_text()
            m = re.search(r"model name\s*:\s*(.+)", raw)
            if m:
                cpu_model = m.group(1).strip()
        else:
            out = _run_cmd("sysctl -n machdep.cpu.brand_string")
            if out:
                cpu_model = out
    except (OSError, UnicodeDecodeError):
        pass
    info["node_cpu_model"] = cpu_model

    # ── CPU count (physical cores / logical cores) ──
    logical = os.cpu_count() or 0
    physical = 0
    try:
        if cpu_info_path.exists():
            raw = cpu_info_path.read_text()
            # Count unique (physical id, core id) pairs to handle multi-socket
            pairs = set(
                re.findall(
                    r"physical id\s*:\s*(\d+)\s*\n.*?core id\s*:\s*(\d+)",
                    raw,
                    re.DOTALL,
                )
            )
            if pairs:
                physical = len(pairs)
            else:
                cores_per_socket = re.search(r"cpu cores\s*:\s*(\d+)", raw)
                if cores_per_socket:
                    physical = int(cores_per_socket.group(1))
        else:
            out = _run_cmd("sysctl -n hw.physicalcpu")
            if out:
                physical = int(out)
    except (OSError, UnicodeDecodeError, ValueError):
        pass
    info["node_cpu_cores"] = str(physical) if physical else str(logical)
    info["node_cpu_threads"] = str(logical)

    # ── Total RAM (GB, rounded to nearest integer) ──
    total_gb = 0
    meminfo = Path("/proc/meminfo")
    try:
        if meminfo.exists():
            raw = meminfo.read_text()
            m = re.search(r"MemTotal:\s*(\d+)\s*kB", raw)
            if m:
                total_gb = round(int(m.group(1)) / 1_048_576)
        else:
            out = _run_cmd("sysctl -n hw.memsize")
            if out:
                total_gb = round(int(out) / (1024 ** 3))
    except (OSError, UnicodeDecodeError, ValueError):
        pass
    info["node_memory_gb"] = str(total_gb) if total_gb else "UNKNOWN"

    # ── CPU base/max frequency (MHz) ──
    freq = "UNKNOWN"
    freq_path = Path("/sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_max_freq")
    try:
        if freq_path.exists():
            khz = int(freq_path.read_text().strip())
            freq = str(round(khz / 1000))
    except (ValueError, OSError):
        pass
    if freq == "UNKNOWN":
        out = _run_cmd("lscpu")
        m = re.search(r"CPU max MHz:\s*([\d.]+)", out)
        if m:
            freq = str(round(float(m.group(1))))
        elif re.search(r"CPU MHz:\s*([\d.]+)", out):
            freq = str(round(float(re.search(r"CPU MHz:\s*([\d.]+)", out).group(1))))
    info["node_cpu_max_mhz"] = freq

    # ── L3 cache size ──
    l3 = "UNKNOWN"
    l3_path = Path("/sys/devices/system/cpu/cpu0/cache/index3/size")
    try:
        if l3_path.exists():
            l3 = l3_path.read_text().strip()
        else:
            out = _run_cmd("lscpu")
            m = re.search(r"L3 cache:\s*(.+)", out)
            if m:
                l3 = m.group(1).strip()
    except (OSError, UnicodeDecodeError):
        pass
    info["node_l3_cache"] = l3

    # ── EC2 instance type (only on AWS — avoids 2s timeout on non-EC2 hosts) ──
    cloud = os.getenv("CLOUD", "").lower()
    if cloud == "aws":
        info["node_instance_type"] = _get_ec2_instance_type()

    return info


def _get_ec2_instance_type() -> str:
    """
    Query the EC2 Instance Metadata Service for the instance type.
    Tries IMDSv2 (token-based) first, falls back to IMDSv1.
    Returns 'UNKNOWN' on non-EC2 hosts or any failure.
    Short timeouts (1s) ensure this never blocks on non-AWS machines.
    """
    import urllib.request
    import urllib.error

    metadata_url = "http://169.254.169.254/latest/meta-data/instance-type"

    # Try IMDSv2 first (requires a session token)
    token_url = "http://169.254.169.254/latest/api/token"
    try:
        token_req = urllib.request.Request(
            token_url,
            method="PUT",
            headers={"X-aws-ec2-metadata-token-ttl-seconds": "30"},
        )
        with urllib.request.urlopen(token_req, timeout=1) as resp:
            token = resp.read().decode().strip()

        req = urllib.request.Request(
            metadata_url,
            headers={"X-aws-ec2-metadata-token": token},
        )
        with urllib.request.urlopen(req, timeout=1) as resp:
            return resp.read().decode().strip()
    except Exception:
        pass

    # Fallback to IMDSv1
    try:
        with urllib.request.urlopen(metadata_url, timeout=1) as resp:
            return resp.read().decode().strip()
    except Exception:
        return "UNKNOWN"


def log_node_info(node_info: dict) -> None:
    """Log collected node hardware information."""
    logger.info("")
    logger.info("=" * 70)
    logger.info("HOST NODE HARDWARE INFO")
    logger.info("=" * 70)
    for key, value in node_info.items():
        label = key.replace("node_", "").replace("_", " ").title()
        logger.info(f"  {label:<20}: {value}")
    logger.info("=" * 70)
    logger.info("")
