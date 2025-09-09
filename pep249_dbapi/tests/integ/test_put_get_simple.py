import tempfile
from pathlib import Path

from .utils_put_get import (
    as_file_uri,
    write_text_file,
    decompress_gzip_file,
    create_temporary_stage,
    PUT_ROW_SOURCE_IDX,
    PUT_ROW_TARGET_IDX,
    PUT_ROW_SOURCE_SIZE_IDX,
    PUT_ROW_TARGET_SIZE_IDX,
    PUT_ROW_SOURCE_COMPRESSION_IDX,
    PUT_ROW_TARGET_COMPRESSION_IDX,
    PUT_ROW_STATUS_IDX,
    PUT_ROW_MESSAGE_IDX,
    GET_ROW_FILE_IDX,
    GET_ROW_SIZE_IDX,
    GET_ROW_STATUS_IDX,
    GET_ROW_MESSAGE_IDX,
    LS_ROW_NAME_IDX,
)

from ..connector_types import ConnectorType


def test_put_select(cursor):
    stage_name = create_temporary_stage(cursor, "PYTEST_STAGE_PUT_SELECT")

    # Create temporary CSV file
    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir_path = Path(tmpdir)
        file_path = write_text_file(tmpdir_path, "test_put_select.csv", "1,2,3\n")

        # Upload the file to the stage
        cursor.execute(
            f"PUT 'file://{as_file_uri(file_path)}' @{stage_name}"
        )

        # Query the staged file and verify the content
        select_sql = f"SELECT $1, $2, $3 FROM @{stage_name}"
        cursor.execute(select_sql)
        row = cursor.fetchone()
        assert row == ("1", "2", "3")


def test_put_ls(cursor):
    stage_name = create_temporary_stage(cursor, "PYTEST_STAGE_PUT_LS")
    filename = "test_put_ls.csv"

    # Create temporary CSV file
    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir_path = Path(tmpdir)
        file_path = write_text_file(tmpdir_path, filename, "1,2,3\n")

        # Upload the file to the stage
        cursor.execute(
            f"PUT 'file://{as_file_uri(file_path)}' @{stage_name}"
        )

        # List stage contents and verify the gzipped file is present
        expected_filename = f"{stage_name.lower()}/{filename}.gz"
        cursor.execute(f"LS @{stage_name}")
        row = cursor.fetchone()

        assert row[LS_ROW_NAME_IDX] == expected_filename


def test_get(cursor):
    stage_name = create_temporary_stage(cursor, "PYTEST_STAGE_GET")
    filename = "test_get.csv"

    # Create temporary CSV file
    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir_path = Path(tmpdir)
        file_path = write_text_file(tmpdir_path, filename, "1,2,3\n")

        # Upload the file to the stage
        cursor.execute(
            f"PUT 'file://{as_file_uri(file_path)}' @{stage_name}"
        )

        download_dir = tmpdir_path / "download"
        download_dir.mkdir(parents=True, exist_ok=True)

        # Download with GET into local directory
        cursor.execute(
            f"GET @{stage_name}/{filename} 'file://{as_file_uri(download_dir)}/'"
        )

        # Expect gzipped file at destination
        expected_file = download_dir / f"{filename}.gz"
        assert expected_file.exists(), f"Expected downloaded file at {expected_file}"

        # Decompress and compare content
        decompressed = decompress_gzip_file(expected_file)
        original = file_path.read_text()
        assert decompressed == original


def test_put_get_rowset(cursor, connector_type):
    stage_name = create_temporary_stage(cursor, "PYTEST_STAGE_PUT_ROWSET")
    filename = "test_put_get_rowset.csv"

    # Create temporary CSV file
    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir_path = Path(tmpdir)
        file_path = write_text_file(tmpdir_path, filename, "1,2,3\n")

        # Upload the file to the stage
        cursor.execute(f"PUT 'file://{as_file_uri(file_path)}' @{stage_name}")

        # Verify the upload result
        row = cursor.fetchone()
        assert row[PUT_ROW_SOURCE_IDX] == "test_put_get_rowset.csv"
        assert row[PUT_ROW_TARGET_IDX] == "test_put_get_rowset.csv.gz"
        assert row[PUT_ROW_SOURCE_SIZE_IDX] == 6

        # BREAKING CHANGE: changing the compression behavior reduces the size of a compressed file
        if connector_type == ConnectorType.REFERENCE:
            assert row[PUT_ROW_TARGET_SIZE_IDX] == 64
        else:
            assert row[PUT_ROW_TARGET_SIZE_IDX] == 32

        assert row[PUT_ROW_SOURCE_COMPRESSION_IDX] == "NONE"
        assert row[PUT_ROW_TARGET_COMPRESSION_IDX] == "GZIP"
        assert row[PUT_ROW_STATUS_IDX] == "UPLOADED"
        assert row[PUT_ROW_MESSAGE_IDX] == ""

        # Download the file from the stage
        cursor.execute(f"GET @{stage_name}/{filename} 'file://{as_file_uri(tmpdir_path)}/'")

        # Verify the download result
        row = cursor.fetchone()

        assert row[GET_ROW_FILE_IDX] == "test_put_get_rowset.csv.gz"

        # BREAKING CHANGE: changing the compression behavior reduces the size of a compressed file
        if connector_type == ConnectorType.REFERENCE:
            assert row[GET_ROW_SIZE_IDX] == 52
        else:
            assert row[GET_ROW_SIZE_IDX] == 26

        assert row[GET_ROW_STATUS_IDX] == "DOWNLOADED"
        assert row[GET_ROW_MESSAGE_IDX] == ""
