import logging
from pathlib import Path

logger = logging.getLogger(__name__)


def verify_results(
    results_dir: Path,
    test_name: str,
    driver: str,
    min_iterations: int = 1,
    driver_type: str = None,
) -> list[Path]:
    """
    Verify that result files were created and contain expected data.
    
    Args:
        results_dir: Directory containing results
        test_name: Name of the test
        driver: Driver name
        min_iterations: Minimum number of iterations expected
        driver_type: Driver type (universal/old), if applicable
    
    Returns:
        List of result file paths
        
    Raises:
        RuntimeError: If no result files are found or they're invalid
    """
    # For drivers with type variants, include driver type in pattern
    # Core only has universal implementation
    if driver != "core" and driver_type:
        pattern = f"{test_name}_{driver}_{driver_type}_*.csv"
    else:
        pattern = f"{test_name}_{driver}_*.csv"
    
    result_files = list(results_dir.glob(pattern))
    
    if len(result_files) == 0:
        raise RuntimeError(f"No result files found matching pattern '{pattern}' in {results_dir}")
    
    # Verify CSV structure
    latest_file = max(result_files, key=lambda p: p.stat().st_mtime)
    try:
        with open(latest_file, 'r') as f:
            lines = f.readlines()
            if len(lines) <= 1:
                raise RuntimeError(f"Result file {latest_file} is empty or missing data")
            if "timestamp" not in lines[0] or "query_s" not in lines[0]:
                raise RuntimeError(f"Invalid CSV header in {latest_file}")
            # +1 for header line
            if len(lines) < min_iterations + 1:
                raise RuntimeError(
                    f"Expected at least {min_iterations} data rows, got {len(lines) - 1}"
                )
    except IOError as e:
        raise RuntimeError(f"Failed to read result file {latest_file}: {e}")
    
    return result_files


