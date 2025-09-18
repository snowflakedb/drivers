import tempfile
from pathlib import Path

from .utils_put_get import (
    as_file_uri,
    create_temporary_stage,
)
from ..utils import shared_test_data_dir
from ..compatibility import OLD_DRIVER_ONLY, NEW_DRIVER_ONLY


def test_put_get_with_auto_compress_true(cursor):
    stage_name = create_temporary_stage(cursor, "PYTEST_STAGE_PUT_GET_COMPRESS_TRUE")

    uncompressed_filename, uncompressed_file_path = uncompressed_test_file()
    compressed_filename, compressed_file_path = compressed_test_file()

    cursor.execute(
        f"PUT 'file://{as_file_uri(uncompressed_file_path)}' @{stage_name} AUTO_COMPRESS=TRUE"
    )

    with tempfile.TemporaryDirectory() as download_dir:
        download_dir_path = Path(download_dir)

        # Download the compressed file
        cursor.execute(
            f"GET @{stage_name}/{uncompressed_filename} 'file://{as_file_uri(download_dir_path)}/'"
        )

        expected_file_path = download_dir_path / compressed_filename
        assert expected_file_path.exists()

        not_expected_file_path = download_dir_path / uncompressed_filename
        assert not not_expected_file_path.exists()

        # Verify content
        downloaded_content = expected_file_path.read_bytes()
        reference_content = compressed_file_path.read_bytes()

        if OLD_DRIVER_ONLY("BC#1"):
            # Old driver adds extra gzip headers, compressed content should differ
            assert downloaded_content != reference_content

        if NEW_DRIVER_ONLY("BC#1"):
            # New driver matches gzip content exactly
            assert downloaded_content == reference_content


def test_put_get_with_auto_compress_false(cursor):
    stage_name = create_temporary_stage(cursor, "PYTEST_STAGE_PUT_GET_COMPRESS_FALSE")

    uncompressed_filename, uncompressed_file_path = uncompressed_test_file()
    compressed_filename, compressed_file_path = compressed_test_file()

    # Upload the uncompressed file with AUTO_COMPRESS=FALSE
    cursor.execute(
        f"PUT 'file://{as_file_uri(uncompressed_file_path)}' @{stage_name} AUTO_COMPRESS=FALSE"
    )

    with tempfile.TemporaryDirectory() as download_dir:
        download_dir_path = Path(download_dir)

        # Download the uncompressed file
        cursor.execute(
            f"GET @{stage_name}/{uncompressed_filename} 'file://{as_file_uri(download_dir_path)}/'"
        )

        # Verify that the downloaded file exists and is uncompressed
        expected_file_path = download_dir_path / uncompressed_filename
        assert expected_file_path.exists()

        not_expected_file_path = download_dir_path / compressed_filename
        assert not not_expected_file_path.exists()

        # Verify that the uncompressed file content is correct
        downloaded_content = expected_file_path.read_text()
        reference_content = uncompressed_file_path.read_text()
        assert downloaded_content == reference_content


def uncompressed_test_file():
    return "test_data.csv", shared_test_data_dir() / "compression" / "test_data.csv"


def compressed_test_file():
    return "test_data.csv.gz", shared_test_data_dir() / "compression" / "test_data.csv.gz"
