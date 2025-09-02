import tempfile
from pathlib import Path

import pytest

from .utils_put_get import (
    write_text_file,
    write_binary_file,
    as_file_uri,
    create_temporary_stage,
    compress_bytes,
    PUT_ROW_SOURCE_IDX,
    PUT_ROW_TARGET_IDX,
    PUT_ROW_SOURCE_COMPRESSION_IDX,
    PUT_ROW_TARGET_COMPRESSION_IDX,
    PUT_ROW_STATUS_IDX,
)


@pytest.mark.parametrize(
    "filename,compression_type",
    [
        ("test_gzip.csv.gz", "GZIP"),
        ("test_bzip2.csv.bz2", "BZ2"),
        ("test_brotli.csv.br", "BROTLI"),
        ("test_zstd.csv.zst", "ZSTD"),
        ("test_deflate.csv.deflate", "DEFLATE"),
    ],
)
def test_put_source_compression_auto_detect_standard_types(cursor, filename, compression_type):
    stage_name = create_temporary_stage(cursor, "PYTEST_STAGE_AUTO_DETECT_STANDARD")
    content = b"1,2,3\n"

    # Create a temporary file and compress it
    with tempfile.TemporaryDirectory() as tmp:
        tmpdir = Path(tmp)
        comp_bytes = compress_bytes(content, compression_type)
        path = write_binary_file(tmpdir, filename, comp_bytes)

        # Upload the compressed file to the stage with AUTO_DETECT
        cursor.execute(
            f"PUT 'file://{as_file_uri(path)}' @{stage_name} SOURCE_COMPRESSION=AUTO_DETECT"
        )

        # Verify that the file was uploaded, compression type was detected correctly and the file was not compressed again
        row = cursor.fetchone()
        assert row[PUT_ROW_SOURCE_IDX] == filename
        assert row[PUT_ROW_TARGET_IDX] == filename
        assert row[PUT_ROW_SOURCE_COMPRESSION_IDX] == compression_type
        assert row[PUT_ROW_TARGET_COMPRESSION_IDX] == compression_type
        assert row[PUT_ROW_STATUS_IDX] == "UPLOADED"


def test_put_source_compression_auto_detect_raw_deflate(cursor):
    stage_name = create_temporary_stage(cursor, "PYTEST_STAGE_AUTO_DETECT_RAW_DEFLATE")
    filename = "test_raw_deflate.csv.raw_deflate"

    # Create a temporary file with .raw_deflate extension
    # Since raw deflate data does not have any headers, we rely on extension for compression type detection
    with tempfile.TemporaryDirectory() as tmp:
        tmpdir = Path(tmp)
        path = write_binary_file(tmpdir, filename, b"rawdeflatedata")

        # Upload the raw deflate file to the stage with AUTO_DETECT
        cursor.execute(
            f"PUT 'file://{as_file_uri(path)}' @{stage_name} SOURCE_COMPRESSION=AUTO_DETECT"
        )

        # Verify that the file was uploaded, compression type was detected correctly and the file was not compressed again
        row = cursor.fetchone()
        assert row[PUT_ROW_SOURCE_IDX] == filename
        assert row[PUT_ROW_TARGET_IDX] == filename
        assert row[PUT_ROW_SOURCE_COMPRESSION_IDX] == "RAW_DEFLATE"
        assert row[PUT_ROW_TARGET_COMPRESSION_IDX] == "RAW_DEFLATE"
        assert row[PUT_ROW_STATUS_IDX] == "UPLOADED"


def test_put_source_compression_auto_detect_none_no_auto_compress(cursor):
    stage_name = create_temporary_stage(cursor, "PYTEST_STAGE_AUTO_DETECT_NONE_NO_AUTO_COMPRESS")
    filename = "test_none.csv"

    # Create a temporary file with .csv extension
    with tempfile.TemporaryDirectory() as tmp:
        tmpdir = Path(tmp)
        path = write_text_file(tmpdir, filename, "1,2,3\n")

        # Upload the file to the stage with AUTO_DETECT and AUTO_COMPRESS=FALSE
        cursor.execute(
            f"PUT 'file://{as_file_uri(path)}' @{stage_name} SOURCE_COMPRESSION=AUTO_DETECT AUTO_COMPRESS=FALSE"
        )

        # Verify that the file was uploaded, compression type was detected as "NONE" correctly and the file was not compressed
        row = cursor.fetchone()
        assert row[PUT_ROW_SOURCE_IDX] == filename
        assert row[PUT_ROW_TARGET_IDX] == filename
        assert row[PUT_ROW_SOURCE_COMPRESSION_IDX] == "NONE"
        assert row[PUT_ROW_TARGET_COMPRESSION_IDX] == "NONE"
        assert row[PUT_ROW_STATUS_IDX] == "UPLOADED"


def test_put_source_compression_auto_detect_none_with_auto_compress(cursor):
    stage_name = create_temporary_stage(cursor, "PYTEST_STAGE_AUTO_DETECT_NONE_WITH_AUTO_COMPRESS")
    filename = "test_none.csv"

    # Create a temporary file with .csv extension
    with tempfile.TemporaryDirectory() as tmp:
        tmpdir = Path(tmp)
        path = write_text_file(tmpdir, filename, "1,2,3\n")

        # Upload the file to the stage with AUTO_DETECT and AUTO_COMPRESS=TRUE
        cursor.execute(
            f"PUT 'file://{as_file_uri(path)}' @{stage_name} SOURCE_COMPRESSION=AUTO_DETECT AUTO_COMPRESS=TRUE"
        )

        # Verify that the file was uploaded, compression type was detected as "NONE" correctly and the file was compressed to gzip
        row = cursor.fetchone()
        assert row[PUT_ROW_SOURCE_IDX] == filename
        assert row[PUT_ROW_TARGET_IDX] == f"{filename}.gz"
        assert row[PUT_ROW_SOURCE_COMPRESSION_IDX] == "NONE"
        assert row[PUT_ROW_TARGET_COMPRESSION_IDX] == "GZIP"
        assert row[PUT_ROW_STATUS_IDX] == "UPLOADED"


@pytest.mark.parametrize(
    "filename,compression_type",
    [
        ("test_gzip.csv", "GZIP"),
        ("test_bzip2.csv", "BZ2"),
        ("test_brotli.csv", "BROTLI"),
        ("test_zstd.csv", "ZSTD"),
        ("test_deflate.csv", "DEFLATE"),
    ],
)
def test_put_source_compression_explicit_standard_types(cursor, filename, compression_type):
    stage_name = create_temporary_stage(cursor, "PYTEST_STAGE_EXPLICIT_COMPRESSION")

    # Create a temporary file and compress it with the specified compression type, but without the extension
    with tempfile.TemporaryDirectory() as tmp:
        tmpdir = Path(tmp)
        comp_bytes = compress_bytes(b"1,2,3\n", compression_type)
        path = write_binary_file(tmpdir, filename, comp_bytes)

        # Upload the compressed file to the stage with the specified compression type
        cursor.execute(
            f"PUT 'file://{as_file_uri(path)}' @{stage_name} SOURCE_COMPRESSION={compression_type}"
        )

        # Verify that the file was uploaded, compression type was detected correctly and the file was not compressed again
        row = cursor.fetchone()
        assert row[PUT_ROW_SOURCE_IDX] == filename
        assert row[PUT_ROW_TARGET_IDX] == filename
        assert row[PUT_ROW_SOURCE_COMPRESSION_IDX] == compression_type
        assert row[PUT_ROW_TARGET_COMPRESSION_IDX] == compression_type
        assert row[PUT_ROW_STATUS_IDX] == "UPLOADED"


def test_put_source_compression_explicit_raw_deflate(cursor):
    stage_name = create_temporary_stage(cursor, "PYTEST_STAGE_EXPLICIT_RAW_DEFLATE")
    filename = "test_explicit_raw_deflate"

    # Create a temporary file without the .raw_deflate extension
    with tempfile.TemporaryDirectory() as tmp:
        tmpdir = Path(tmp)
        path = write_binary_file(tmpdir, filename, b"rawdeflatedata")

        # Upload the raw deflate file to the stage with the specified compression type
        cursor.execute(
            f"PUT 'file://{as_file_uri(path)}' @{stage_name} SOURCE_COMPRESSION=RAW_DEFLATE"
        )

        # Verify that the file was uploaded, compression type was detected correctly and the file was not compressed again
        row = cursor.fetchone()
        assert row[PUT_ROW_SOURCE_IDX] == filename
        assert row[PUT_ROW_TARGET_IDX] == filename
        assert row[PUT_ROW_SOURCE_COMPRESSION_IDX] == "RAW_DEFLATE"
        assert row[PUT_ROW_TARGET_COMPRESSION_IDX] == "RAW_DEFLATE"
        assert row[PUT_ROW_STATUS_IDX] == "UPLOADED"


def test_put_source_compression_explicit_none_no_auto_compress(cursor):
    stage_name = create_temporary_stage(cursor, "PYTEST_STAGE_EXPLICIT_NONE_NO_AUTO_COMPRESS")
    filename = "test_explicit_none.csv.gz"

    # Create a temporary uncompressed file with the wrong .gz extension
    with tempfile.TemporaryDirectory() as tmp:
        tmpdir = Path(tmp)
        path = write_text_file(tmpdir, filename, "1,2,3\n")

        # Upload the uncompressed file to the stage with NONE compression type and AUTO_COMPRESS=FALSE
        cursor.execute(
            f"PUT 'file://{as_file_uri(path)}' @{stage_name} SOURCE_COMPRESSION=NONE AUTO_COMPRESS=FALSE"
        )

        # Verify that the file was uploaded, compression type was set to NONE and the file was not compressed
        row = cursor.fetchone()
        assert row[PUT_ROW_SOURCE_IDX] == filename
        assert row[PUT_ROW_TARGET_IDX] == filename
        assert row[PUT_ROW_SOURCE_COMPRESSION_IDX] == "NONE"
        assert row[PUT_ROW_TARGET_COMPRESSION_IDX] == "NONE"
        assert row[PUT_ROW_STATUS_IDX] == "UPLOADED"


def test_put_source_compression_explicit_with_auto_compress(cursor):
    stage_name = create_temporary_stage(cursor, "PYTEST_STAGE_EXPLICIT_WITH_AUTO_COMPRESS")
    filename = "test_explicit_with_auto.csv.gz"

    # Create a temporary uncompressed file with the wrong .gz extension
    with tempfile.TemporaryDirectory() as tmp:
        tmpdir = Path(tmp)
        path = write_text_file(tmpdir, filename, "1,2,3\n")

        # Upload the uncompressed file to the stage with NONE compression type and AUTO_COMPRESS=TRUE
        cursor.execute(
            f"PUT 'file://{as_file_uri(path)}' @{stage_name} SOURCE_COMPRESSION=NONE AUTO_COMPRESS=TRUE"
        )

        # Verify that the file was uploaded, compression type was set to NONE and the file was compressed to gzip
        row = cursor.fetchone()
        assert row[PUT_ROW_SOURCE_IDX] == filename
        assert row[PUT_ROW_TARGET_IDX] == f"{filename}.gz"
        assert row[PUT_ROW_SOURCE_COMPRESSION_IDX] == "NONE"
        assert row[PUT_ROW_TARGET_COMPRESSION_IDX] == "GZIP"
        assert row[PUT_ROW_STATUS_IDX] == "UPLOADED"
