"""S3 utilities for downloading test data files."""

import logging
import subprocess
import shutil
from pathlib import Path

logger = logging.getLogger(__name__)


def download_s3_files(s3_url: str, local_dir: Path) -> int:
    """
    Download files from S3 to local directory using aws s3 cp.
    
    Skips download if files are already present.
    The files are then mounted into driver containers for PUT/GET operations.
    
    Args:
        s3_url: S3 URL (e.g., s3://sfc-eng-data/ecosystem/12Mx100/)
        local_dir: Local directory path to download files to
    
    Returns:
        Number of files present (either already downloaded or newly downloaded)
        
    Raises:
        RuntimeError: If AWS CLI is not available or download fails
    """
    # Check if files are already present
    if local_dir.exists():
        existing_files = list(local_dir.glob("*"))
        if existing_files:
            file_count = len(existing_files)
            logger.info(f"✓ Files already present in {local_dir} ({file_count} files)")
            logger.info(f"  Skipping S3 download. Delete directory to re-download.")
            return file_count
        else:
            # Directory exists but is empty, remove it
            logger.info(f"Removing empty directory: {local_dir}")
            shutil.rmtree(local_dir)
    
    local_dir.mkdir(parents=True, exist_ok=True)
    
    logger.info(f"Downloading files from {s3_url} to {local_dir}...")
    
    try:
        subprocess.run(
            [
                "aws", "s3", "cp",
                "--recursive",
                "--only-show-errors",
                s3_url,
                str(local_dir)
            ],
            check=True,
            capture_output=True,
            text=True,
            timeout=300  # 5 minute timeout
        )
        
        # Count downloaded files
        file_count = len(list(local_dir.glob("*")))
        logger.info(f"✓ Downloaded {file_count} files from S3")
        
        return file_count
        
    except subprocess.TimeoutExpired:
        logger.error(f"S3 download timed out after 5 minutes")
        raise RuntimeError("S3 download timed out")
        
    except subprocess.CalledProcessError as e:
        logger.error(f"S3 download failed with exit code {e.returncode}")
        if e.stdout:
            logger.error(f"STDOUT: {e.stdout}")
        if e.stderr:
            logger.error(f"STDERR: {e.stderr}")
        raise RuntimeError(f"S3 download failed: {e.stderr}")
        
    except FileNotFoundError:
        logger.error("AWS CLI not found. Please install aws-cli.")
        raise RuntimeError("AWS CLI is not installed")
