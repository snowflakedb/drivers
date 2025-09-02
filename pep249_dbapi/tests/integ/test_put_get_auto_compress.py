import tempfile
from pathlib import Path

from .utils_put_get import (
    write_text_file,
    as_file_uri,
    create_temporary_stage,
    decompress_gzip_file,
    PUT_ROW_SOURCE_IDX,
    PUT_ROW_TARGET_IDX,
    PUT_ROW_STATUS_IDX,
    GET_ROW_FILE_IDX,
    GET_ROW_SIZE_IDX,
    GET_ROW_STATUS_IDX,
    GET_ROW_MESSAGE_IDX,
)


def test_put_get_with_auto_compress_true(cursor):
    stage_name = create_temporary_stage(cursor, "PYTEST_STAGE_PUT_GET_COMPRESS_TRUE")
    filename = "test_put_get_compress_true.csv"
    compressed_filename = f"{filename}.gz"

    # Create temporary uncompressed CSV file
    with tempfile.TemporaryDirectory() as tmp:
        tmpdir = Path(tmp)
        file_path = write_text_file(tmpdir, filename, "1,2,3\n")

        # Upload the uncompressed file with AUTO_COMPRESS=TRUE
        cursor.execute(
            f"PUT 'file://{as_file_uri(file_path)}' @{stage_name} AUTO_COMPRESS=TRUE"
        )

        # Verify that the file was uploaded and compressed
        row = cursor.fetchone()
        assert row[PUT_ROW_SOURCE_IDX] == filename
        assert row[PUT_ROW_TARGET_IDX] == compressed_filename
        assert row[PUT_ROW_STATUS_IDX] == "UPLOADED"

        download_dir = tmpdir / "download"
        download_dir.mkdir(parents=True, exist_ok=True)

        # Download the compressed file
        cursor.execute(
            f"GET @{stage_name}/{filename} 'file://{as_file_uri(download_dir)}/'"
        )
        
        # Verify that the compressed file has correct name
        row = cursor.fetchone()
        assert row[GET_ROW_FILE_IDX] == compressed_filename
        assert row[GET_ROW_STATUS_IDX] == "DOWNLOADED"

        # Verify that the compressed file exists and the uncompressed file does not
        gz = download_dir / compressed_filename
        assert gz.exists()

        plain = download_dir / filename
        assert not plain.exists()

        # Verify that the compressed file content is correct
        assert decompress_gzip_file(gz) == "1,2,3\n"


def test_put_get_with_auto_compress_false(cursor):
    stage_name = create_temporary_stage(cursor, "PYTEST_STAGE_PUT_GET_COMPRESS_FALSE")
    filename = "test_put_get_compress_false.csv"
    compressed_filename = f"{filename}.gz"
    
    # Create temporary uncompressed CSV file
    with tempfile.TemporaryDirectory() as tmp:
        tmpdir = Path(tmp)
        file_path = write_text_file(tmpdir, filename, "1,2,3\n")

        # Upload the uncompressed file with AUTO_COMPRESS=FALSE
        cursor.execute(
            f"PUT 'file://{as_file_uri(file_path)}' @{stage_name} AUTO_COMPRESS=FALSE"
        )

        # Verify that the file was uploaded and not compressed
        row = cursor.fetchone()
        assert row[PUT_ROW_SOURCE_IDX] == filename
        assert row[PUT_ROW_TARGET_IDX] == filename
        assert row[PUT_ROW_STATUS_IDX] == "UPLOADED"

        download_dir = tmpdir / "download"
        download_dir.mkdir(parents=True, exist_ok=True)

        # Download the uncompressed file
        cursor.execute(
            f"GET @{stage_name}/{filename} 'file://{as_file_uri(download_dir)}/'"
        )

        # Verify that the uncompressed file exists and the compressed file does not
        row = cursor.fetchone()
        assert row[GET_ROW_FILE_IDX] == filename
        assert row[GET_ROW_STATUS_IDX] == "DOWNLOADED"

        plain = download_dir / filename
        assert plain.exists()
        gz = download_dir / compressed_filename
        assert not gz.exists()

        # Verify that the uncompressed file content is correct
        assert plain.read_text() == "1,2,3\n"


#################################################################################################
# More tests with upload of compressed files can be found in test_put_get_source_compression.py #
#################################################################################################
