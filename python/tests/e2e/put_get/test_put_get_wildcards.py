import tempfile
from pathlib import Path

from tests.e2e.put_get.put_get_helper import (
    as_file_uri,
    create_matching_files,
    create_temporary_stage,
    create_test_files,
    list_stage_contents,
    upload_file_to_stage,
)


def test_should_upload_files_that_match_wildcard_question_mark_pattern(connection):
    base_file_name = "test_put_wildcard_question_mark"

    with tempfile.TemporaryDirectory() as temp_dir, connection.cursor() as cursor:
        temp_path = Path(temp_dir)

        # Given Files matching wildcard pattern
        matching_files = create_matching_files(temp_path, base_file_name)

        # And Files not matching wildcard pattern
        non_matching_files = [
            f"{base_file_name}_10.csv",
            f"{base_file_name}_abc.csv",
        ]
        create_test_files(temp_path, non_matching_files)

        # When Files are uploaded using command with question mark wildcard
        wildcard_pattern = (temp_path / f"{base_file_name}_?.csv").as_posix()
        stage_name, upload_results = create_temporary_stage_and_upload_multiple_files(
            cursor,
            "TEST_PUT_WILDCARD_QUESTION_MARK",
            wildcard_pattern,
            auto_compress=False,
            overwrite=True,
        )

        # Then Files matching wildcard pattern are uploaded
        assert len(upload_results) == 5

        stage_contents = list_stage_contents(cursor, stage_name)
        uploaded_filenames = [Path(item[0]).name for item in stage_contents]

        for filename in matching_files:
            assert filename in uploaded_filenames

        # And Files not matching wildcard pattern are not uploaded
        for filename in non_matching_files:
            assert filename not in uploaded_filenames


def test_should_upload_files_that_match_wildcard_star_pattern(connection):
    base_file_name = "test_put_wildcard_star"

    with tempfile.TemporaryDirectory() as temp_dir, connection.cursor() as cursor:
        temp_path = Path(temp_dir)

        # Given Files matching wildcard pattern
        matching_files = create_matching_files(temp_path, base_file_name)

        # And Files not matching wildcard pattern
        non_matching_files = [
            f"{base_file_name}.csv",
            f"{base_file_name}_test.txt",
        ]
        create_test_files(temp_path, non_matching_files)

        # When Files are uploaded using command with star wildcard
        wildcard_pattern = (temp_path / f"{base_file_name}_*.csv").as_posix()
        stage_name, upload_results = create_temporary_stage_and_upload_multiple_files(
            cursor,
            "TEST_PUT_WILDCARD_STAR",
            wildcard_pattern,
            auto_compress=False,
            overwrite=True,
        )

        # Then Files matching wildcard pattern are uploaded
        assert len(upload_results) == 5

        stage_contents = list_stage_contents(cursor, stage_name)
        uploaded_filenames = [Path(item[0]).name for item in stage_contents]

        for filename in matching_files:
            assert filename in uploaded_filenames

        # And Files not matching wildcard pattern are not uploaded
        for filename in non_matching_files:
            assert filename not in uploaded_filenames


def test_should_download_files_that_are_matching_wildcard_pattern(connection):
    base_file_name = "test_get"

    with tempfile.TemporaryDirectory() as temp_dir, connection.cursor() as cursor:
        temp_path = Path(temp_dir)

        # Given Files matching wildcard pattern are uploaded
        matching_files = create_matching_files(temp_path, base_file_name)
        stage_name = create_temporary_stage(cursor, "TEST_GET_WILDCARD")
        for filename in matching_files:
            file_path = temp_path / filename
            upload_file_to_stage(
                cursor, stage_name, file_path, auto_compress=True, overwrite=True
            )

        # And Files not matching wildcard pattern are uploaded
        non_matching_files = [
            f"{base_file_name}_10.csv",
            f"{base_file_name}_abc.csv",
        ]
        create_test_files(temp_path, non_matching_files)
        for filename in non_matching_files:
            file_path = temp_path / filename
            upload_file_to_stage(
                cursor, stage_name, file_path, auto_compress=True, overwrite=True
            )

        with tempfile.TemporaryDirectory() as download_temp_dir:
            download_dir = Path(download_temp_dir)

            # When Files are downloaded using command with wildcard
            pattern = f".*/{base_file_name}_.\\.csv\\.gz"
            get_files_with_wildcard(cursor, stage_name, pattern, download_dir)

            # Then Files matching wildcard pattern are downloaded
            downloaded_files = list(download_dir.iterdir())
            assert len(downloaded_files) == 5
            downloaded_filenames = [f.name for f in downloaded_files]

            matching_files_gz = [f"{f}.gz" for f in matching_files]
            for filename in matching_files_gz:
                assert filename in downloaded_filenames

            # And Files not matching wildcard pattern are not downloaded
            non_matching_files_gz = [f"{f}.gz" for f in non_matching_files]
            for filename in non_matching_files_gz:
                assert filename not in downloaded_filenames


def upload_files_with_wildcard(
    cursor,
    stage_name: str,
    wildcard_pattern: str,
    auto_compress: bool = True,
    overwrite: bool = True,
):
    """
    Upload files matching a wildcard pattern to a Snowflake stage.

    Args:
        cursor: Database cursor to execute the command
        stage_name: Name of the existing stage to upload to
        wildcard_pattern: Wildcard pattern for files to upload (e.g., 'pattern_?.csv')
        auto_compress: Whether to enable auto compression (default: True)
        overwrite: Whether to overwrite existing files (default: True)

    Returns:
        list: List of result rows from the PUT command (one per uploaded file)
    """
    options_str = (
        f"AUTO_COMPRESS={str(auto_compress).upper()} OVERWRITE={str(overwrite).upper()}"
    )
    put_command = f"PUT 'file://{wildcard_pattern}' @{stage_name} {options_str}"
    cursor.execute(put_command)
    return cursor.fetchall()


def create_temporary_stage_and_upload_multiple_files(
    cursor,
    stage_prefix: str,
    wildcard_pattern: str,
    auto_compress: bool = True,
    overwrite: bool = True,
):
    """
    Function that creates temporary stage and uploads multiple files using wildcard pattern.

    Args:
        cursor: Database cursor to use for operations
        stage_prefix: Prefix for the temporary stage name
        wildcard_pattern: Wildcard pattern for files to upload (e.g., '/path/to/files/*.csv')
        auto_compress: Whether to enable auto compression for upload (default: True)
        overwrite: Whether to overwrite existing files for upload (default: True)

    Returns:
        tuple: (stage_name, upload_results)

    Note:
        All uploads are automatically validated for success.
    """
    stage_name = create_temporary_stage(cursor, stage_prefix)
    upload_results = upload_files_with_wildcard(
        cursor, stage_name, wildcard_pattern, auto_compress, overwrite
    )
    for upload_result in upload_results:
        assert (
            upload_result[6] == "UPLOADED"
        ), f"File upload failed. Status: {upload_result[6]}"

    return stage_name, upload_results


def get_files_with_wildcard(cursor, stage_name: str, pattern: str, download_dir: Path):
    """
    Download files matching a regex pattern from a Snowflake stage.

    Args:
        cursor: Database cursor to execute the command
        stage_name: Name of the stage to download from
        pattern: Regex pattern for files to download (e.g., '.*pattern_.\\.csv\\.gz')
        download_dir: Local directory to download files to

    Note:
        This function executes the GET command but does not return results.
        Check the download_dir for downloaded files after calling this function.
    """
    get_command = (
        f"GET @{stage_name} 'file://{as_file_uri(download_dir)}/' PATTERN='{pattern}'"
    )
    cursor.execute(get_command)
