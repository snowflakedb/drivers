"""Local binary execution for performance tests"""
import json
import logging
import os
import shutil
import subprocess
from pathlib import Path
from runner.test_types import TestType

logger = logging.getLogger(__name__)

_get_files_cleaned = False


def run_local_core_binary(
    test_name: str,
    sql_command: str,
    parameters_json: str,
    results_dir: Path,
    iterations: int,
    warmup_iterations: int,
    setup_queries: list[str] = None,
    test_type: TestType = TestType.SELECT,
    s3_files_dir: Path = None,
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
        test_type: Type of test (TestType.SELECT or TestType.PUT_GET)
        s3_files_dir: Optional directory with S3-downloaded files (for PUT/GET tests)
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
    
    if test_type == TestType.PUT_GET and s3_files_dir:
        get_files_dir = repo_root / "tests" / "performance" / "get_files"
        
        global _get_files_cleaned
        if not _get_files_cleaned:
            if get_files_dir.exists():
                logger.info("")
                logger.info("=" * 80)
                logger.info(f"🧹 Cleaning GET files directory (local execution): {get_files_dir}")
                logger.info("=" * 80)
                logger.info("")
                shutil.rmtree(get_files_dir)
            _get_files_cleaned = True
        
        # Rewrite source path: /put_get_files/ → actual S3 download directory
        docker_source_path = "file:///put_get_files/"
        local_source_path = f"file://{s3_files_dir.absolute()}/"
        
        # Rewrite destination path: /get_files/ → project local directory
        docker_dest_path = "file:///get_files/"
        local_dest_path = f"file://{get_files_dir.absolute()}/"
        
        # Rewrite SQL command (both source and destination)
        sql_command = sql_command.replace(docker_source_path, local_source_path)
        sql_command = sql_command.replace(docker_dest_path, local_dest_path)
        
        # Rewrite setup queries (both source and destination)
        if setup_queries:
            setup_queries = [
                q.replace(docker_source_path, local_source_path).replace(docker_dest_path, local_dest_path)
                for q in setup_queries
            ]
        
        logger.info(f"Rewriting Docker paths:")
        logger.info(f"  Source:      /put_get_files/ → {s3_files_dir.absolute()}")
        logger.info(f"  Destination: /get_files/ → {get_files_dir.absolute()}")
    
    # Prepare environment variables
    env = os.environ.copy()
    env["TEST_NAME"] = test_name
    env["SQL_COMMAND"] = sql_command
    env["PARAMETERS_JSON"] = parameters_json
    env["PERF_ITERATIONS"] = str(iterations)
    env["PERF_WARMUP_ITERATIONS"] = str(warmup_iterations)
    env["RESULTS_DIR"] = str(results_dir.absolute())
    env["TEST_TYPE"] = test_type.value
    
    if "RUST_VERSION" not in env:
        try:
            rustc_output = subprocess.run(
                ["rustc", "--version"],
                capture_output=True,
                text=True,
                timeout=5
            )
            if rustc_output.returncode == 0:
                parts = rustc_output.stdout.split()
                if len(parts) >= 2:
                    version = parts[1]
                    major_minor = '.'.join(version.split('.')[:2])
                    env["RUST_VERSION"] = major_minor
        except Exception as e:
            env["RUST_VERSION"] = "unknown"
    
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
