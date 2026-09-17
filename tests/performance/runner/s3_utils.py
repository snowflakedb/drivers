"""S3 utilities for downloading test data files."""

import logging
import subprocess
import shutil
from pathlib import Path

logger = logging.getLogger(__name__)

_WRITE_CHUNK_SIZE = 64 * 1024


def ensure_local_put_get_files(
    local_dir: Path,
    file_size_bytes: int,
    file_count: int = 1,
    filename_prefix: str = "file",
) -> int:
    """
    Create fixed-size local files for PUT/GET perf tests.

    Skips creation when local_dir already holds file_count files of the
    requested size. Removes local_dir on failure so a partial write is not reused.
    """
    if file_count < 1:
        raise ValueError(f"file_count must be >= 1, got {file_count}")
    if file_size_bytes < 1:
        raise ValueError(f"file_size_bytes must be >= 1, got {file_size_bytes}")

    expected_names = {f"{filename_prefix}_{index + 1}.bin" for index in range(file_count)}

    if local_dir.exists():
        existing = [path for path in local_dir.iterdir() if path.is_file()]
        if (
            len(existing) == file_count
            and {path.name for path in existing} == expected_names
            and all(path.stat().st_size == file_size_bytes for path in existing)
        ):
            logger.info(
                f"✓ Local PUT/GET files already present in {local_dir} "
                f"({file_count} x {file_size_bytes} bytes)"
            )
            return file_count
        logger.info(f"Local PUT/GET cache miss in {local_dir}, recreating files.")
        shutil.rmtree(local_dir)

    local_dir.mkdir(parents=True, exist_ok=True)
    chunk = b"\0" * _WRITE_CHUNK_SIZE

    try:
        for index in range(file_count):
            path = local_dir / f"{filename_prefix}_{index + 1}.bin"
            remaining = file_size_bytes
            with path.open("wb") as handle:
                while remaining > 0:
                    write_size = min(remaining, len(chunk))
                    handle.write(chunk[:write_size])
                    remaining -= write_size
        logger.info(
            f"✓ Created {file_count} local PUT/GET file(s) in {local_dir} "
            f"({file_size_bytes} bytes each)"
        )
        return file_count
    except OSError:
        if local_dir.exists():
            shutil.rmtree(local_dir)
        raise


def download_s3_files(s3_url: str, local_dir: Path, max_files: int | None = None) -> int:
    """
    Download files from S3 to local directory using aws s3 cp.
    
    Skips download when the directory already has a complete cache: any files
    if max_files is unset, or exactly max_files entries otherwise. A failed
    download deletes local_dir so a partial copy is not reused.
    The files are then mounted into driver containers for PUT/GET operations.
    
    Args:
        s3_url: S3 URL (e.g., s3://sfc-eng-data/ecosystem/12Mx100/)
        local_dir: Local directory path to download files to
        max_files: If set, copy only the first N objects from the prefix
    
    Returns:
        Number of files present (either already downloaded or newly downloaded)
        
    Raises:
        RuntimeError: If AWS CLI is not available or download fails
    """
    if local_dir.exists():
        existing_files = list(local_dir.glob("*"))
        if existing_files:
            file_count = len(existing_files)
            if max_files is None or file_count == max_files:
                logger.info(f"✓ Files already present in {local_dir} ({file_count} files)")
                logger.info(f"  Skipping S3 download. Delete directory to re-download.")
                return file_count
            logger.info(
                f"Cached file count {file_count} != max_files={max_files} in {local_dir}, "
                f"re-downloading."
            )
            shutil.rmtree(local_dir)
        else:
            logger.info(f"Removing empty directory: {local_dir}")
            shutil.rmtree(local_dir)

    local_dir.mkdir(parents=True, exist_ok=True)

    logger.info(f"Downloading files from {s3_url} to {local_dir}...")

    download_ok = False
    try:
        if max_files is None:
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
        else:
            _download_s3_prefix_subset(s3_url, local_dir, max_files)

        file_count = len(list(local_dir.glob("*")))
        logger.info(f"✓ Downloaded {file_count} files from S3")
        download_ok = True
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

    finally:
        if not download_ok and local_dir.exists():
            shutil.rmtree(local_dir)


def _download_s3_prefix_subset(s3_url: str, local_dir: Path, max_files: int) -> None:
    keys = _list_s3_object_keys(s3_url)
    if not keys:
        raise RuntimeError(f"No objects found under {s3_url}")
    prefix = s3_url.rstrip("/") + "/"
    for name in keys[:max_files]:
        subprocess.run(
            [
                "aws", "s3", "cp",
                "--only-show-errors",
                prefix + name,
                str(local_dir / name),
            ],
            check=True,
            capture_output=True,
            text=True,
            timeout=300,
        )


def _list_s3_object_keys(s3_url: str) -> list[str]:
    result = subprocess.run(
        ["aws", "s3", "ls", s3_url],
        check=True,
        capture_output=True,
        text=True,
        timeout=60,
    )
    keys = []
    for line in result.stdout.splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("PRE "):
            continue
        parts = stripped.split(None, 3)
        if len(parts) == 4:
            keys.append(parts[3])
    return keys
