import logging
import json
import time
import threading
from pathlib import Path
from testcontainers.core.container import DockerContainer

logger = logging.getLogger(__name__)


def create_perf_container(
    driver: str,
    parameters_json: str,
    sql_command: str,
    test_name: str,
    iterations: int,
    warmup_iterations: int,
    results_dir: Path,
    driver_type: str = None,
    setup_queries: list[str] = None,
) -> DockerContainer:
    """
    Create and configure a Docker container for performance testing.
    
    Args:
        driver: Driver name (core, python, odbc, jdbc)
        parameters_json: JSON string with connection parameters
        sql_command: SQL command to execute
        test_name: Name of the test
        iterations: Number of test iterations
        warmup_iterations: Number of warmup iterations
        results_dir: Directory to mount for results
        driver_type: Driver type: 'universal' or 'old' (only 'universal' for core)
        setup_queries: Optional list of SQL queries to run before warmup/test iterations
    
    Returns:
        Configured DockerContainer instance
    """
    image_name = f"{driver}-perf-driver:latest"
    
    container = (
        DockerContainer(image_name)
        .with_env("PARAMETERS_JSON", parameters_json)
        .with_env("SQL_COMMAND", sql_command)
        .with_env("TEST_NAME", test_name)
        .with_env("PERF_ITERATIONS", str(iterations))
        .with_env("PERF_WARMUP_ITERATIONS", str(warmup_iterations))
        .with_volume_mapping(str(results_dir), "/results", mode="rw")
    )
    
    if setup_queries:
        container = container.with_env("SETUP_QUERIES", json.dumps(setup_queries))
    
    if driver != "core" and driver_type:
        container = container.with_env("DRIVER_TYPE", driver_type)
    
    return container


def run_container(container: DockerContainer) -> str:
    """
    Run a Docker container.
    
    Args:
        container: Configured DockerContainer instance
    
    Returns:
        Container logs as string
    """
    with container:
        result = container.get_wrapped_container()
        
        timeout = 1800  # 30 minutes timeout
        start_time = time.time()
        logs_buffer = []
        stream_error = None
        
        def stream_logs():
            """Stream logs from container in background thread"""
            nonlocal stream_error
            try:
                for log_chunk in result.logs(stream=True, follow=True):
                    log_line = log_chunk.decode('utf-8').rstrip('\n')
                    if log_line.strip():
                        logger.info(log_line)
                        logs_buffer.append(log_line)
            except Exception as e:
                stream_error = e
        
        # Start log streaming in background thread
        log_thread = threading.Thread(target=stream_logs, daemon=True)
        log_thread.start()
        
        # Wait for container to finish with timeout
        while result.status != 'exited':
            elapsed = time.time() - start_time
            if elapsed > timeout:
                logger.error(f"Container timed out after {timeout}s")
                raise TimeoutError(f"Container execution exceeded {timeout}s")
            
            result.reload()
            time.sleep(0.5)
        
        # Wait for log thread to finish
        log_thread.join(timeout=5)
        
        if stream_error:
            logger.warning(f"Log streaming error: {stream_error}")
        
        # Get final logs
        logs_combined = result.logs().decode('utf-8')
        exit_code = result.attrs.get('State', {}).get('ExitCode', 0)
        
        if exit_code != 0:
            logger.error(f"\nContainer exited with code {exit_code}")
            # If streaming failed, show all logs now
            if stream_error or not logs_buffer:
                logger.error("="*80)
                logger.error("FULL CONTAINER OUTPUT:")
                logger.error("="*80)
                for line in logs_combined.splitlines():
                    if line.strip():
                        logger.error(line)
                logger.error("="*80)
        
    return logs_combined


