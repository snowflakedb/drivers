"""
Helper functions for PUT/GET operations in e2e tests.
"""
import uuid
from pathlib import Path


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
    Convert a file path to a file URI format suitable for Snowflake commands.
    
    Args:
        file_path: Path object to convert
        
    Returns:
        str: File path in URI format
    """
    return file_path.as_posix().replace("\\", "/")


def upload_file_to_stage(cursor, stage_name: str, file_path: Path,
                         auto_compress: bool = True, overwrite: bool = True):
    """
    Upload a file to an existing Snowflake stage using a cursor.
    
    Args:
        cursor: Database cursor to execute the command
        stage_name: Name of the existing stage to upload to
        file_path: Path to the file to upload
        auto_compress: Whether to enable auto compression (default: True)
        overwrite: Whether to overwrite existing files (default: True)
        
    Returns:
        The raw result row from the PUT command - caller can analyze as needed
    """
    # Build PUT command with options
    file_uri = as_file_uri(file_path)
    put_options = []
    
    if auto_compress:
        put_options.append("AUTO_COMPRESS=TRUE")
    else:
        put_options.append("AUTO_COMPRESS=FALSE")
        
    if overwrite:
        put_options.append("OVERWRITE=TRUE")
    else:
        put_options.append("OVERWRITE=FALSE")
    
    options_str = " ".join(put_options)
    put_command = f"PUT 'file://{file_uri}' @{stage_name} {options_str}"

    # Execute PUT command
    cursor.execute(put_command)
    return cursor.fetchone()


def list_stage_contents(cursor, stage_name: str) -> list:
    """
    List the contents of a Snowflake stage using a cursor.
    
    Args:
        cursor: Database cursor to execute the command
        stage_name: Name of the stage to list
        
    Returns:
        list: List of files in the stage, each row contains file information
    """
    ls_command = f"LS @{stage_name}"
    cursor.execute(ls_command)
    
    # Fetch all rows from the LS command result
    rows = cursor.fetchall()
    return rows


def get_file_from_stage(cursor, stage_name: str, filename: str, download_dir: Path):
    """
    Download a file from a Snowflake stage using a cursor.
    
    Args:
        cursor: Database cursor to execute the command
        stage_name: Name of the stage to download from
        filename: Name of the file to download (without .gz extension)
        download_dir: Local directory to download the file to
        
    Returns:
        The raw result row from the GET command - caller can analyze as needed
    """
    download_uri = as_file_uri(download_dir)
    get_command = f"GET @{stage_name}/{filename} 'file://{download_uri}/'"
    
    # Execute GET command and return raw result
    cursor.execute(get_command)
    return cursor.fetchone()