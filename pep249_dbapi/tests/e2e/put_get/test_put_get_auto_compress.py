import tempfile
from pathlib import Path

from tests.compatibility import NEW_DRIVER_ONLY, OLD_DRIVER_ONLY
from tests.e2e.put_get.put_get_helper import (
    get_file_from_stage,
    create_temporary_stage_and_upload_file,
)
from tests.utils import shared_test_data_dir


def test_should_compress_the_file_before_uploading_to_stage_when_auto_compress_set_to_true(
    connection,
):
    uncompressed_file_path = shared_test_data_dir() / "compression" / "test_data.csv"
    compressed_file_path = shared_test_data_dir() / "compression" / "test_data.csv.gz"
    uncompressed_filename = "test_data.csv"
    compressed_filename = "test_data.csv.gz"
    with connection.cursor() as cursor:

        # Given Snowflake client is logged in
        assert cursor is not None

        # When File is uploaded to stage with AUTO_COMPRESS set to true
        stage_name, _ = create_temporary_stage_and_upload_file(
            cursor,
            "TEST_PUT_GET_AUTO_COMPRESS_TRUE",
            uncompressed_file_path,
            auto_compress=True,
            overwrite=True,
        )

        with tempfile.TemporaryDirectory() as temp_dir:

            # Then Only compressed file should be downloaded
            download_dir = Path(temp_dir)

            get_result = get_file_from_stage(
                cursor, stage_name, uncompressed_filename, download_dir
            )

            assert get_result[2] == "DOWNLOADED"

            expected_file_path = download_dir / compressed_filename
            assert expected_file_path.exists()

            not_expected_file_path = download_dir / uncompressed_filename
            assert not not_expected_file_path.exists()

            # And Have correct content
            downloaded_content = expected_file_path.read_bytes()
            reference_content = compressed_file_path.read_bytes()

            if OLD_DRIVER_ONLY("BC#1"):
                assert downloaded_content != reference_content

            if NEW_DRIVER_ONLY("BC#1"):
                assert downloaded_content == reference_content


def test_should_not_compress_the_file_before_uploading_to_stage_when_auto_compress_set_to_false(
    connection,
):
    uncompressed_file_path = shared_test_data_dir() / "compression" / "test_data.csv"
    uncompressed_filename = "test_data.csv"
    compressed_filename = "test_data.csv.gz"

    with connection.cursor() as cursor:

        # Given Snowflake client is logged in
        assert cursor is not None

        # When File is uploaded to stage with AUTO_COMPRESS set to false
        stage_name, _ = create_temporary_stage_and_upload_file(
            cursor,
            "TEST_PUT_GET_AUTO_COMPRESS_FALSE",
            uncompressed_file_path,
            auto_compress=False,
            overwrite=True,
        )

        with tempfile.TemporaryDirectory() as temp_dir:

            # Then Only uncompressed file should be downloaded
            download_dir = Path(temp_dir)
            get_result = get_file_from_stage(
                cursor, stage_name, uncompressed_filename, download_dir
            )

            assert get_result[2] == "DOWNLOADED"

            expected_file_path = download_dir / uncompressed_filename
            assert expected_file_path.exists()

            not_expected_file_path = download_dir / compressed_filename
            assert not not_expected_file_path.exists()

            # And Have correct content
            downloaded_content = expected_file_path.read_bytes()
            reference_content = uncompressed_file_path.read_bytes()
            assert downloaded_content == reference_content
