import pytest
from pathlib import Path

from tests.e2e.put_get.put_get_helper import (
    as_file_uri,
    create_temporary_stage,
)
from tests.compatibility import NEW_DRIVER_ONLY, OLD_DRIVER_ONLY
from tests.utils import shared_test_data_dir


@pytest.mark.parametrize(
    "expected_compression,filename",
    [
        ("GZIP", "test_data.csv.gz"),
        ("BZIP2", "test_data.csv.bz2"),
        ("BROTLI", "test_data.csv.br"),
        ("ZSTD", "test_data.csv.zst"),
        ("DEFLATE", "test_data.csv.deflate"),
    ],
)
def test_should_auto_detect_standard_compression_types_when_source_compression_set_to_auto_detect(
    connection, expected_compression, filename
):
    with connection.cursor() as cursor:
        # Given Snowflake client is logged in
        assert cursor is not None

        # And File with standard type (GZIP, BZIP2, BROTLI, ZSTD, DEFLATE)
        stage_name, test_file_path = create_stage_and_get_compression_file(
            cursor, f"TEST_STAGE_{expected_compression}", expected_compression
        )

        # When File is uploaded with SOURCE_COMPRESSION set to AUTO_DETECT
        put_command = f"PUT 'file://{as_file_uri(test_file_path)}' @{stage_name} SOURCE_COMPRESSION=AUTO_DETECT"
        cursor.execute(put_command)
        result = cursor.fetchone()

        # Then Target compression has correct type and all PUT results are correct
        if expected_compression == "DEFLATE":
            if OLD_DRIVER_ONLY("BD#2"):
                expected_target = f"{filename}.gz"
                assert_put_compression_result(
                    result,
                    filename,
                    "NONE",
                    expected_target,
                    "GZIP",
                )
            elif NEW_DRIVER_ONLY("BD#2"):
                assert_put_compression_result(
                    result,
                    filename,
                    expected_compression,
                    filename,
                    expected_compression,
                )
        else:
            assert_put_compression_result(
                result,
                filename,
                expected_compression,
                filename,
                expected_compression,
            )


@pytest.mark.parametrize(
    "compression,filename",
    [
        ("GZIP", "test_data.csv.gz"),
        ("BZIP2", "test_data.csv.bz2"),
        ("BROTLI", "test_data.csv.br"),
        ("ZSTD", "test_data.csv.zst"),
        ("DEFLATE", "test_data.csv.deflate"),
        ("RAW_DEFLATE", "test_data.csv.raw_deflate"),
    ],
)
def test_should_upload_compressed_files_with_source_compression_set_to_explicit_types(
    connection, compression, filename
):
    with connection.cursor() as cursor:
        # Given Snowflake client is logged in
        assert cursor is not None

        # And File with standard type (GZIP, BZIP2, BROTLI, ZSTD, DEFLATE, RAW_DEFLATE)
        stage_name, test_file_path = create_stage_and_get_compression_file(
            cursor, f"TEST_STAGE_{compression}", compression
        )

        # When File is uploaded with SOURCE_COMPRESSION set to explicit type
        put_command = f"PUT 'file://{as_file_uri(test_file_path)}' @{stage_name} SOURCE_COMPRESSION={compression}"

        if compression == "BROTLI":
            if OLD_DRIVER_ONLY("BD#3"):
                with pytest.raises(Exception) as exc_info:
                    cursor.execute(put_command)
                assert "253007" in str(exc_info.value)
                assert "Feature is not supported" in str(exc_info.value)
                return
            elif NEW_DRIVER_ONLY("BD#3"):
                cursor.execute(put_command)
                result = cursor.fetchone()
        else:
            cursor.execute(put_command)
            result = cursor.fetchone()

        # Then Target compression has correct type and all PUT results are correct
        assert_put_compression_result(
            result, filename, compression, filename, compression
        )


def test_should_not_compress_file_when_source_compression_set_to_auto_detect_and_auto_compress_set_to_false(
    connection,
):
    with connection.cursor() as cursor:
        # Given Snowflake client is logged in
        assert cursor is not None

        # And Uncompressed file
        stage_name, test_file_path = create_stage_and_get_compression_file(
            cursor, "TEST_STAGE_NONE_NO_AUTO_COMPRESS", "NONE"
        )
        filename = "test_data.csv"

        # When File is uploaded with SOURCE_COMPRESSION set to AUTO_DETECT and AUTO_COMPRESS set to FALSE
        put_command = f"PUT 'file://{as_file_uri(test_file_path)}' @{stage_name} SOURCE_COMPRESSION=AUTO_DETECT AUTO_COMPRESS=FALSE"
        cursor.execute(put_command)
        result = cursor.fetchone()

        # Then File is not compressed
        assert_put_compression_result(result, filename, "NONE", filename, "NONE")


def test_should_not_compress_file_when_source_compression_set_to_none_and_auto_compress_set_to_false(
    connection,
):
    with connection.cursor() as cursor:
        # Given Snowflake client is logged in
        assert cursor is not None

        # And Uncompressed file
        stage_name, test_file_path = create_stage_and_get_compression_file(
            cursor, "TEST_STAGE_NONE_NO_AUTO_COMPRESS", "NONE"
        )
        filename = "test_data.csv"

        # When File is uploaded with SOURCE_COMPRESSION set to NONE and AUTO_COMPRESS set to FALSE
        put_command = f"PUT 'file://{as_file_uri(test_file_path)}' @{stage_name} SOURCE_COMPRESSION=NONE AUTO_COMPRESS=FALSE"
        cursor.execute(put_command)
        result = cursor.fetchone()

        # Then File is not compressed
        assert_put_compression_result(result, filename, "NONE", filename, "NONE")


def test_should_compress_uncompressed_file_when_source_compression_set_to_auto_detect_and_auto_compress_set_to_true(
    connection,
):
    with connection.cursor() as cursor:
        # Given Snowflake client is logged in
        assert cursor is not None

        # And Uncompressed file
        stage_name, test_file_path = create_stage_and_get_compression_file(
            cursor, "TEST_STAGE_AUTO_COMPRESS", "NONE"
        )
        filename = "test_data.csv"

        # When File is uploaded with SOURCE_COMPRESSION set to AUTO_DETECT and AUTO_COMPRESS set to TRUE
        put_command = f"PUT 'file://{as_file_uri(test_file_path)}' @{stage_name} SOURCE_COMPRESSION=AUTO_DETECT AUTO_COMPRESS=TRUE"
        cursor.execute(put_command)
        result = cursor.fetchone()

        # Then Target compression has GZIP type and all PUT results are correct
        expected_target = f"{filename}.gz"
        assert_put_compression_result(result, filename, "NONE", expected_target, "GZIP")


def test_should_compress_uncompressed_file_when_source_compression_set_to_none_and_auto_compress_set_to_true(
    connection,
):
    with connection.cursor() as cursor:
        # Given Snowflake client is logged in
        assert cursor is not None

        # And Uncompressed file
        stage_name, test_file_path = create_stage_and_get_compression_file(
            cursor, "TEST_STAGE_NONE_AUTO_COMPRESS", "NONE"
        )
        filename = "test_data.csv"

        # When File is uploaded with SOURCE_COMPRESSION set to NONE and AUTO_COMPRESS set to TRUE
        put_command = f"PUT 'file://{as_file_uri(test_file_path)}' @{stage_name} SOURCE_COMPRESSION=NONE AUTO_COMPRESS=TRUE"
        cursor.execute(put_command)
        result = cursor.fetchone()

        # Then Target compression has GZIP type and all PUT results are correct
        expected_target = f"{filename}.gz"
        assert_put_compression_result(result, filename, "NONE", expected_target, "GZIP")


def test_should_return_error_for_unsupported_compression_type(connection):
    with connection.cursor() as cursor:
        # Given Snowflake client is logged in
        assert cursor is not None

        # And File compressed with unsupported format
        stage_name, test_file_path = create_stage_and_get_compression_file(
            cursor, "TEST_STAGE_UNSUPPORTED", "LZMA"
        )
        filename = "test_data.csv.xz"

        # When File is uploaded with SOURCE_COMPRESSION set to AUTO_DETECT
        put_command = f"PUT 'file://{as_file_uri(test_file_path)}' @{stage_name} SOURCE_COMPRESSION=AUTO_DETECT"

        # Then Unsupported compression error is thrown
        with pytest.raises(Exception) as exc_info:
            cursor.execute(put_command)

        if NEW_DRIVER_ONLY("BD#4"):
            assert "Unsupported compression type" in str(exc_info.value)

        if OLD_DRIVER_ONLY("BD#4"):
            assert "253007" in str(exc_info.value)
            assert "Feature is not supported" in str(exc_info.value)


def get_compression_test_file_path(compression_type: str) -> Path:
    """
    Get the path for a test file with the specified compression type.

    Args:
        compression_type: Compression type (GZIP, BZIP2, BROTLI, ZSTD, DEFLATE, RAW_DEFLATE, NONE, LZMA)

    Returns:
        Path: Path to the test file

    Raises:
        ValueError: If compression type is not supported
    """
    compression_map = {
        "GZIP": "test_data.csv.gz",
        "BZIP2": "test_data.csv.bz2",
        "BROTLI": "test_data.csv.br",
        "ZSTD": "test_data.csv.zst",
        "DEFLATE": "test_data.csv.deflate",
        "RAW_DEFLATE": "test_data.csv.raw_deflate",
        "NONE": "test_data.csv",
        "LZMA": "test_data.csv.xz",
    }

    filename = compression_map.get(compression_type.upper())
    if not filename:
        raise ValueError(f"Unsupported compression type: {compression_type}")

    return shared_test_data_dir() / "compression" / filename


def create_stage_and_get_compression_file(
    cursor, stage_prefix: str, compression_type: str
):
    """
    Create a temporary stage and get the compression test file path.

    Args:
        cursor: Database cursor to use
        stage_prefix: Prefix for the temporary stage name
        compression_type: Compression type (GZIP, BZIP2, BROTLI, ZSTD, DEFLATE, RAW_DEFLATE, NONE, LZMA)

    Returns:
        tuple: (stage_name, test_file_path)
    """
    stage_name = create_temporary_stage(cursor, stage_prefix)
    test_file_path = get_compression_test_file_path(compression_type)
    return stage_name, test_file_path


def assert_put_compression_result(
    result,
    expected_source: str,
    expected_source_compression: str,
    expected_target: str,
    expected_target_compression: str,
):
    """
    Assert that PUT result matches expected compression values.

    Args:
        result: PUT command result row
        expected_source: Expected source filename
        expected_source_compression: Expected source compression type
        expected_target: Expected target filename
        expected_target_compression: Expected target compression type
    """
    assert (
        result[0] == expected_source
    ), f"Source should be '{expected_source}', got '{result[0]}'"
    assert (
        result[1] == expected_target
    ), f"Target should be '{expected_target}', got '{result[1]}'"
    assert (
        result[4] == expected_source_compression
    ), f"Source compression should be '{expected_source_compression}', got '{result[4]}'"
    assert (
        result[5] == expected_target_compression
    ), f"Target compression should be '{expected_target_compression}', got '{result[5]}'"
    assert result[6] == "UPLOADED", f"Status should be 'UPLOADED', got '{result[6]}'"
