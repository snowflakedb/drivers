"""
Helper functions for PUT/GET operations in e2e tests.
"""

import uuid
from contextlib import contextmanager
from pathlib import Path
from typing import NamedTuple


class UploadResult(NamedTuple):
    """Result from PUT command with named fields for better readability."""

    source: str
    target: str
    source_size: int
    target_size: int
    source_compression: str
    target_compression: str
    status: str
    message: str


class DownloadResult(NamedTuple):
    """Result from GET command with named fields for better readability."""

    file: str
    size: int
    status: str
    message: str


def create_temporary_stage(cursor, prefix: str) -> str:
    """
    Create a temporary stage with a unique name using UUID.

    Args:
        cursor: Database cursor to execute the command
        prefix: Prefix for the stage name

    Returns:
        str: The name of the created temporary stage
    """
    stage_name = f"{prefix}_{uuid.uuid4().hex}".upper()
    cursor.execute(f"CREATE TEMPORARY STAGE {stage_name}")
    return stage_name


def as_file_uri(file_path: Path) -> str:
    """
    Convert a file path to URI format suitable for Snowflake commands.

    Args:
        file_path: Path object to convert

    Returns:
        str: File path in URI format
    """
    return file_path.as_posix()


def upload_file_to_stage(
    cursor,
    stage_name: str,
    file_path: Path,
    auto_compress: bool = True,
    overwrite: bool = True,
) -> UploadResult:
    """
    Upload a file to an existing Snowflake stage.

    Args:
        cursor: Database cursor to execute the command
        stage_name: Name of the existing stage to upload to
        file_path: Path to the file to upload
        auto_compress: Whether to enable auto compression (default: True)
        overwrite: Whether to overwrite existing files (default: True)

    Returns:
        UploadResult: Named tuple with upload result fields
    """
    file_uri = as_file_uri(file_path)
    options_str = (
        f"AUTO_COMPRESS={str(auto_compress).upper()} OVERWRITE={str(overwrite).upper()}"
    )
    put_command = f"PUT 'file://{file_uri}' @{stage_name} {options_str}"
    cursor.execute(put_command)
    raw_result = cursor.fetchone()
    return UploadResult(*raw_result)


def list_stage_contents(cursor, stage_name: str) -> list:
    """
    List the contents of a Snowflake stage.

    Args:
        cursor: Database cursor to execute the command
        stage_name: Name of the stage to list

    Returns:
        list: List of files in the stage with file information
    """
    ls_command = f"LS @{stage_name}"
    cursor.execute(ls_command)
    return cursor.fetchall()


def get_file_from_stage(
    cursor, stage_name: str, filename: str, download_dir: Path
) -> DownloadResult:
    """
    Download a file from a Snowflake stage.

    Args:
        cursor: Database cursor to execute the command
        stage_name: Name of the stage to download from
        filename: Name of the file to download (without .gz extension)
        download_dir: Local directory to download the file to

    Returns:
        DownloadResult: Named tuple with download result fields
    """
    download_uri = as_file_uri(download_dir)
    get_command = f"GET @{stage_name}/{filename} 'file://{download_uri}/'"
    cursor.execute(get_command)
    raw_result = cursor.fetchone()
    return DownloadResult(*raw_result)


@contextmanager
def put_get_test_setup(
    connection,
    stage_prefix: str,
    file_path: Path,
    auto_compress: bool = True,
    overwrite: bool = True,
):
    """
    Context manager that provides cursor, temporary stage, and uploads file for PUT/GET tests.

    Args:
        connection: Database connection object
        stage_prefix: Prefix for the temporary stage name
        file_path: Path to file to upload to the stage
        auto_compress: Whether to enable auto compression for upload (default: True)
        overwrite: Whether to overwrite existing files for upload (default: True)

    Yields:
        tuple: (cursor, stage_name, upload_result)

    Note:
        Upload is automatically validated for success.
    """
    with connection.cursor() as cursor:
        stage_name = create_temporary_stage(cursor, stage_prefix)
        upload_result = upload_file_to_stage(
            cursor, stage_name, file_path, auto_compress, overwrite
        )
        assert (
            upload_result.status == "UPLOADED"
        ), f"File upload failed. Status: {upload_result.status}"

        yield cursor, stage_name, upload_result
