"""
Helper functions for PUT/GET operations in e2e tests.
"""

import uuid
from pathlib import Path
from typing import List


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
    cursor.execute(f"CREATE TEMPORARY STAGE IF NOT EXISTS {stage_name}")
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
):
    """
    Upload a file to an existing Snowflake stage.

    Args:
        cursor: Database cursor to execute the command
        stage_name: Name of the existing stage to upload to
        file_path: Path to the file to upload
        auto_compress: Whether to enable auto compression (default: True)
        overwrite: Whether to overwrite existing files (default: True)

    Returns:
        Raw result row from the PUT command
    """
    file_uri = as_file_uri(file_path)
    options_str = (
        f"AUTO_COMPRESS={str(auto_compress).upper()} OVERWRITE={str(overwrite).upper()}"
    )
    put_command = f"PUT 'file://{file_uri}' @{stage_name} {options_str}"
    cursor.execute(put_command)
    return cursor.fetchone()


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


def get_file_from_stage(cursor, stage_name: str, filename: str, download_dir: Path):
    """
    Download a file from a Snowflake stage.

    Args:
        cursor: Database cursor to execute the command
        stage_name: Name of the stage to download from
        filename: Name of the file to download (without .gz extension)
        download_dir: Local directory to download the file to

    Returns:
        Raw result row from the GET command
    """
    download_uri = as_file_uri(download_dir)
    get_command = f"GET @{stage_name}/{filename} 'file://{download_uri}/'"
    cursor.execute(get_command)
    return cursor.fetchone()


def create_temporary_stage_and_upload_file(
    cursor,
    stage_prefix: str,
    file_path: Path,
    auto_compress: bool = True,
    overwrite: bool = True,
):
    """
    Function that creates temporary stage and uploads file for PUT/GET tests.

    Args:
        cursor: Database cursor to use for operations
        stage_prefix: Prefix for the temporary stage name
        file_path: Path to file to upload to the stage
        auto_compress: Whether to enable auto compression for upload (default: True)
        overwrite: Whether to overwrite existing files for upload (default: True)

    Returns:
        tuple: (stage_name, upload_result)

    Note:
        Upload is automatically validated for success.
    """
    stage_name = create_temporary_stage(cursor, stage_prefix)
    upload_result = upload_file_to_stage(
        cursor, stage_name, file_path, auto_compress, overwrite
    )
    assert (
        upload_result[6] == "UPLOADED"
    ), f"File upload failed. Status: {upload_result[6]}"

    return stage_name, upload_result


def create_test_file(directory: Path, filename: str, content: str = "1,2,3\n") -> Path:
    """
    Create a test file with specified content.

    Args:
        directory: Directory where the file should be created
        filename: Name of the file to create
        content: Content to write to the file (default: "1,2,3\n")

    Returns:
        Path: Path to the created file
    """
    directory.mkdir(parents=True, exist_ok=True)
    file_path = directory / filename
    file_path.write_text(content)
    return file_path


def create_test_files(
    directory: Path, filenames: List[str], content: str = "1,2,3\n"
) -> List[Path]:
    """
    Create multiple test files with the same content.

    Args:
        directory: Directory where files should be created
        filenames: List of filenames to create
        content: Content to write to each file (default: "1,2,3\n")

    Returns:
        List[Path]: List of paths to the created files
    """
    return [create_test_file(directory, filename, content) for filename in filenames]


def create_matching_files(
    directory: Path, base_file_name: str, count: int = 5
) -> List[str]:
    """
    Create matching test files with numbered suffixes.

    Args:
        directory: Directory where files should be created
        base_file_name: Base name for the files (e.g., "test_put_wildcard")
        count: Number of files to create (default: 5)

    Returns:
        List[str]: List of created filenames
    """
    matching_files = [f"{base_file_name}_{i}.csv" for i in range(1, count + 1)]
    create_test_files(directory, matching_files)
    return matching_files
