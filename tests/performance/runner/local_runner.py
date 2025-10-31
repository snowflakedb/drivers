"""Local binary execution for performance tests"""
import logging
import json
import subprocess
import os
from pathlib import Path

logger = logging.getLogger(__name__)


def run_local_core_binary(
    test_name: str,
    sql_command: str,
    parameters_json: str,
    results_dir: Path,
    iterations: int,
    warmup_iterations: int,
    setup_queries: list[str] = None,
) -> None:
    """
    Run the locally built Core binary directly (no Docker).
    
    Args:
        test_name: Name of the test
        sql_command: SQL command to execute
        parameters_json: JSON string with connection parameters
        results_dir: Directory to store results
        iterations: Number of test iterations
        warmup_iterations: Number of warmup iterations
        setup_queries: Optional list of SQL queries to run before warmup/test iterations
    """
    repo_root = Path(__file__).parent.parent.parent.parent
    core_app_manifest = repo_root / "tests" / "performance" / "drivers" / "core" / "app" / "Cargo.toml"
    
    logger.info("Building Core binary (release mode)...")
    
    build_result = subprocess.run(
        ["cargo", "build", "--release", "--manifest-path", str(core_app_manifest)],
        cwd=repo_root,
        capture_output=True,
        text=True,
    )
    
    if build_result.returncode != 0:
        logger.error("Failed to build Core binary:")
        logger.error(build_result.stderr)
        raise RuntimeError(f"Cargo build failed with exit code {build_result.returncode}")
    
    # Prepare environment variables
    env = os.environ.copy()
    env["TEST_NAME"] = test_name
    env["SQL_COMMAND"] = sql_command
    env["PARAMETERS_JSON"] = parameters_json
    env["PERF_ITERATIONS"] = str(iterations)
    env["PERF_WARMUP_ITERATIONS"] = str(warmup_iterations)
    env["RESULTS_DIR"] = str(results_dir.absolute())
    
    if setup_queries:
        env["SETUP_QUERIES"] = json.dumps(setup_queries)
    
    # Run the binary
    target_dir = repo_root / "tests" / "performance" / "drivers" / "core" / "app" / "target" / "release"
    binary_path = target_dir / "core-perf-driver"
    
    result = subprocess.run(
        [str(binary_path)],
        cwd=repo_root,
        env=env,
        capture_output=True,
        text=True,
    )
    
    if result.stdout:
        print(result.stdout)
    
    if result.returncode != 0:
        logger.error("Core binary execution failed:")
        if result.stderr:
            logger.error(result.stderr)
        raise RuntimeError(f"Core binary failed with exit code {result.returncode}")
