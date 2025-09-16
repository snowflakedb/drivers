import tempfile
from pathlib import Path

from .utils_put_get import (
    as_file_uri,
    create_temporary_stage,
    shared_test_data_dir,
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

from ..utils import NEW_DRIVER_ONLY, OLD_DRIVER_ONLY


def test_put_select(cursor):
    stage_name = create_temporary_stage(cursor, "PYTEST_STAGE_PUT_SELECT")
    _, file_path = uncompressed_test_file()

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
    filename, file_path = uncompressed_test_file()

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
    filename, file_path = uncompressed_test_file()
    _, compressed_file_path = compressed_test_file()

    # Upload the file to the stage
    cursor.execute(
        f"PUT 'file://{as_file_uri(file_path)}' @{stage_name}"
    )

    with tempfile.TemporaryDirectory() as tmp:
        download_dir = Path(tmp) / "download"
        download_dir.mkdir(parents=True, exist_ok=True)

        # Download with GET into local directory
        cursor.execute(
            f"GET @{stage_name}/{filename} 'file://{as_file_uri(download_dir)}/'"
        )

        # Expect gzipped file at destination
        expected_file = download_dir / f"{filename}.gz"
        assert expected_file.exists(), f"Expected downloaded file at {expected_file}"

        # Verify content
        downloaded_content = expected_file.read_bytes()
        reference_content = compressed_file_path.read_bytes()

        if OLD_DRIVER_ONLY("BC#1"):
            # Old driver adds extra gzip headers, compressed content should differ
            assert downloaded_content != reference_content

        if NEW_DRIVER_ONLY("BC#1"):
            # New driver matches gzip content exactly
            assert downloaded_content == reference_content


def test_put_get_rowset(cursor):
    stage_name = create_temporary_stage(cursor, "PYTEST_STAGE_PUT_ROWSET")
    filename, file_path = uncompressed_test_file()

    # Upload the file to the stage
    cursor.execute(f"PUT 'file://{as_file_uri(file_path)}' @{stage_name}")

    # Verify the upload result
    row = cursor.fetchone()
    assert row[PUT_ROW_SOURCE_IDX] == filename
    assert row[PUT_ROW_TARGET_IDX] == f"{filename}.gz"
    assert row[PUT_ROW_SOURCE_SIZE_IDX] == 6

    if OLD_DRIVER_ONLY("BC#1"):
        assert row[PUT_ROW_TARGET_SIZE_IDX] == 48

    if NEW_DRIVER_ONLY("BC#1"):
        assert row[PUT_ROW_TARGET_SIZE_IDX] == 32

    assert row[PUT_ROW_SOURCE_COMPRESSION_IDX] == "NONE"
    assert row[PUT_ROW_TARGET_COMPRESSION_IDX] == "GZIP"
    assert row[PUT_ROW_STATUS_IDX] == "UPLOADED"
    assert row[PUT_ROW_MESSAGE_IDX] == ""

    with tempfile.TemporaryDirectory() as tmp:
        tmpdir_path = Path(tmp)
        # Download the file from the stage
        cursor.execute(f"GET @{stage_name}/{filename} 'file://{as_file_uri(tmpdir_path)}/'")

        # Verify the download result
        row = cursor.fetchone()

        assert row[GET_ROW_FILE_IDX] == f"{filename}.gz"

        if OLD_DRIVER_ONLY("BC#1"):
            assert row[GET_ROW_SIZE_IDX] == 42

        if NEW_DRIVER_ONLY("BC#1"):
            assert row[GET_ROW_SIZE_IDX] == 26

        assert row[GET_ROW_STATUS_IDX] == "DOWNLOADED"
        assert row[GET_ROW_MESSAGE_IDX] == ""


def uncompressed_test_file():
    return "test_data.csv", shared_test_data_dir() / "compression" / "test_data.csv"


def compressed_test_file():
    return "test_data.csv.gz", shared_test_data_dir() / "compression" / "test_data.csv.gz"
