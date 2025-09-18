import pytest

from ..compatibility import NEW_DRIVER_ONLY, OLD_DRIVER_ONLY
from .utils_put_get import (
    as_file_uri,
    create_temporary_stage,
    PUT_ROW_SOURCE_IDX,
    PUT_ROW_TARGET_IDX,
    PUT_ROW_SOURCE_COMPRESSION_IDX,
    PUT_ROW_TARGET_COMPRESSION_IDX,
    PUT_ROW_STATUS_IDX,
)
from ..utils import shared_test_data_dir


@pytest.mark.parametrize("compression_type", ["GZIP", "BZIP2", "BROTLI", "ZSTD"])
def test_put_source_compression_auto_detect_standard_types(cursor, compression_type):
    stage_name = create_temporary_stage(cursor, "PYTEST_STAGE_AUTO_DETECT_STANDARD")
    filename, file_path = compressed_test_file(compression_type)

    # Upload the compressed file to the stage with AUTO_DETECT
    cursor.execute(
        f"PUT 'file://{as_file_uri(file_path)}' @{stage_name} SOURCE_COMPRESSION=AUTO_DETECT"
    )

    # Verify that the file was uploaded, compression type was detected correctly and the file was not compressed again
    row = cursor.fetchone()
    assert row[PUT_ROW_SOURCE_IDX] == filename
    assert row[PUT_ROW_TARGET_IDX] == filename
    assert row[PUT_ROW_SOURCE_COMPRESSION_IDX] == compression_type
    assert row[PUT_ROW_TARGET_COMPRESSION_IDX] == compression_type
    assert row[PUT_ROW_STATUS_IDX] == "UPLOADED"


def test_put_source_compression_auto_detect_deflate(cursor):
    stage_name = create_temporary_stage(cursor, "PYTEST_STAGE_AUTO_DETECT_DEFLATE")
    filename, file_path = compressed_test_file("DEFLATE")

    # Upload the compressed file to the stage with AUTO_DETECT
    cursor.execute(
        f"PUT 'file://{as_file_uri(file_path)}' @{stage_name} SOURCE_COMPRESSION=AUTO_DETECT"
    )

    row = cursor.fetchone()

    if OLD_DRIVER_ONLY("BC#2"):
        assert row[PUT_ROW_SOURCE_IDX] == filename
        assert row[PUT_ROW_TARGET_IDX] == filename + ".gz"
        assert row[PUT_ROW_SOURCE_COMPRESSION_IDX] == "NONE"
        assert row[PUT_ROW_TARGET_COMPRESSION_IDX] == "GZIP"
        assert row[PUT_ROW_STATUS_IDX] == "UPLOADED"

    if NEW_DRIVER_ONLY("BC#2"):
        assert row[PUT_ROW_SOURCE_IDX] == filename
        assert row[PUT_ROW_TARGET_IDX] == filename
        assert row[PUT_ROW_SOURCE_COMPRESSION_IDX] == "DEFLATE"
        assert row[PUT_ROW_TARGET_COMPRESSION_IDX] == "DEFLATE"
        assert row[PUT_ROW_STATUS_IDX] == "UPLOADED"


def test_put_source_compression_auto_detect_none_no_auto_compress(cursor):
    stage_name = create_temporary_stage(cursor, "PYTEST_STAGE_AUTO_DETECT_NONE_NO_AUTO_COMPRESS")
    filename, file_path = compressed_test_file("NONE")

    # Upload the file to the stage with AUTO_DETECT and AUTO_COMPRESS=FALSE
    cursor.execute(
        f"PUT 'file://{as_file_uri(file_path)}' @{stage_name} SOURCE_COMPRESSION=AUTO_DETECT AUTO_COMPRESS=FALSE"
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
    filename, file_path = compressed_test_file("NONE")

    # Upload the file to the stage with AUTO_DETECT and AUTO_COMPRESS=TRUE
    cursor.execute(
        f"PUT 'file://{as_file_uri(file_path)}' @{stage_name} SOURCE_COMPRESSION=AUTO_DETECT AUTO_COMPRESS=TRUE"
    )

    # Verify that the file was uploaded, compression type was detected as "NONE" correctly and the file was compressed to gzip
    row = cursor.fetchone()
    assert row[PUT_ROW_SOURCE_IDX] == filename
    assert row[PUT_ROW_TARGET_IDX] == f"{filename}.gz"
    assert row[PUT_ROW_SOURCE_COMPRESSION_IDX] == "NONE"
    assert row[PUT_ROW_TARGET_COMPRESSION_IDX] == "GZIP"
    assert row[PUT_ROW_STATUS_IDX] == "UPLOADED"


@pytest.mark.parametrize("compression_type", ["GZIP", "BZIP2", "ZSTD", "DEFLATE", "RAW_DEFLATE"])
def test_put_source_compression_explicit_standard_types(cursor, compression_type):
    stage_name = create_temporary_stage(cursor, "PYTEST_STAGE_EXPLICIT_COMPRESSION")
    filename, file_path = compressed_test_file(compression_type)

    # Upload the compressed file to the stage with the specified compression type
    cursor.execute(
        f"PUT 'file://{as_file_uri(file_path)}' @{stage_name} SOURCE_COMPRESSION={compression_type}"
    )

    # Verify that the file was uploaded, compression type was detected correctly and the file was not compressed again
    row = cursor.fetchone()
    assert row[PUT_ROW_SOURCE_IDX] == filename
    assert row[PUT_ROW_TARGET_IDX] == filename
    assert row[PUT_ROW_SOURCE_COMPRESSION_IDX] == compression_type
    assert row[PUT_ROW_TARGET_COMPRESSION_IDX] == compression_type
    assert row[PUT_ROW_STATUS_IDX] == "UPLOADED"


def test_put_source_compression_explicit_brotli(cursor):
    stage_name = create_temporary_stage(cursor, "PYTEST_STAGE_EXPLICIT_BROTLI")
    filename, file_path = compressed_test_file("BROTLI")

    if OLD_DRIVER_ONLY("BC#3"):
        with pytest.raises(Exception):
            cursor.execute(
                f"PUT 'file://{as_file_uri(file_path)}' @{stage_name} SOURCE_COMPRESSION=BROTLI"
            )

    if NEW_DRIVER_ONLY("BC#3"):
        cursor.execute(
            f"PUT 'file://{as_file_uri(file_path)}' @{stage_name} SOURCE_COMPRESSION=BROTLI"
        )
        row = cursor.fetchone()
        assert row[PUT_ROW_SOURCE_IDX] == filename
        assert row[PUT_ROW_TARGET_IDX] == filename
        assert row[PUT_ROW_SOURCE_COMPRESSION_IDX] == "BROTLI"
        assert row[PUT_ROW_TARGET_COMPRESSION_IDX] == "BROTLI"
        assert row[PUT_ROW_STATUS_IDX] == "UPLOADED"


def test_put_source_compression_explicit_none_no_auto_compress(cursor):
    stage_name = create_temporary_stage(cursor, "PYTEST_STAGE_EXPLICIT_NONE_NO_AUTO_COMPRESS")
    filename, file_path = compressed_test_file("NONE")

    cursor.execute(
        f"PUT 'file://{as_file_uri(file_path)}' @{stage_name} SOURCE_COMPRESSION=NONE AUTO_COMPRESS=FALSE"
    )

    row = cursor.fetchone()
    assert row[PUT_ROW_SOURCE_IDX] == filename
    assert row[PUT_ROW_TARGET_IDX] == filename
    assert row[PUT_ROW_SOURCE_COMPRESSION_IDX] == "NONE"
    assert row[PUT_ROW_TARGET_COMPRESSION_IDX] == "NONE"
    assert row[PUT_ROW_STATUS_IDX] == "UPLOADED"


def test_put_source_compression_explicit_with_auto_compress(cursor):
    stage_name = create_temporary_stage(cursor, "PYTEST_STAGE_EXPLICIT_WITH_AUTO_COMPRESS")
    filename, file_path = compressed_test_file("NONE")

    cursor.execute(
        f"PUT 'file://{as_file_uri(file_path)}' @{stage_name} SOURCE_COMPRESSION=NONE AUTO_COMPRESS=TRUE"
    )

    row = cursor.fetchone()
    assert row[PUT_ROW_SOURCE_IDX] == filename
    assert row[PUT_ROW_TARGET_IDX] == f"{filename}.gz"
    assert row[PUT_ROW_SOURCE_COMPRESSION_IDX] == "NONE"
    assert row[PUT_ROW_TARGET_COMPRESSION_IDX] == "GZIP"
    assert row[PUT_ROW_STATUS_IDX] == "UPLOADED"


def compression_tests_dir():
    return shared_test_data_dir() / "compression"


def compressed_test_file(compression_type: str):
    ct = compression_type.upper()
    base = compression_tests_dir()
    if ct == "GZIP":
        fn = "test_data.csv.gz"
    elif ct == "BZIP2":
        fn = "test_data.csv.bz2"
    elif ct == "BROTLI":
        fn = "test_data.csv.br"
    elif ct == "ZSTD":
        fn = "test_data.csv.zst"
    elif ct == "DEFLATE":
        fn = "test_data.csv.deflate"
    elif ct == "RAW_DEFLATE":
        fn = "test_data.csv.raw_deflate"
    elif ct == "LZMA":
        fn = "test_data.csv.xz"
    elif ct == "NONE":
        fn = "test_data.csv"
    else:
        raise ValueError(f"Unsupported compression type: {compression_type}")
    return fn, base / fn
