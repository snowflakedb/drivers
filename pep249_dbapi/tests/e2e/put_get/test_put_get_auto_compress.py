import pytest
import tempfile
from pathlib import Path

from tests.compatibility import NEW_DRIVER_ONLY, OLD_DRIVER_ONLY
from tests.e2e.put_get.put_get_helper import create_temporary_stage, upload_file_to_stage, get_file_from_stage
from tests.utils import shared_test_data_dir


def test_should_compress_the_file_before_uploading_to_stage_when_auto_compress_set_to_true(connection):
    """Test that should compress the file before uploading to stage when AUTO_COMPRESS set to true."""
    uncompressed_file_path = shared_test_data_dir() / "compression" / "test_data.csv"
    compressed_file_path = shared_test_data_dir() / "compression" / "test_data.csv.gz"
    uncompressed_filename = "test_data.csv"
    compressed_filename = "test_data.csv.gz"
    # Given Snowflake client is logged in
    with connection.cursor() as cursor:
        
        stage_name = create_temporary_stage(cursor, "TEST_PUT_GET_AUTO_COMPRESS_TRUE")

        # When File is uploaded to stage with AUTO_COMPRESS set to true
        upload_result = upload_file_to_stage(
            cursor,
            stage_name,
            uncompressed_file_path,
            auto_compress=True,
            overwrite=True
        )

        assert upload_result[6] == "UPLOADED"

        with tempfile.TemporaryDirectory() as temp_dir:
            download_dir = Path(temp_dir)
            
            get_result = get_file_from_stage(cursor, stage_name, uncompressed_filename, download_dir)
            
            assert get_result[2] == "DOWNLOADED"
            
            # Then Only compressed file should be downloaded
            expected_file_path = download_dir / compressed_filename
            assert expected_file_path.exists(), f"Compressed file should exist at {expected_file_path}"
            
            not_expected_file_path = download_dir / uncompressed_filename
            assert not not_expected_file_path.exists(), f"Uncompressed file should not exist at {not_expected_file_path}"
            
            # And Have correct content
            downloaded_content = expected_file_path.read_bytes()
            reference_content = compressed_file_path.read_bytes()
            
            if OLD_DRIVER_ONLY("BC#1"):
                assert downloaded_content != reference_content
            
            if NEW_DRIVER_ONLY("BC#1"):
                assert downloaded_content == reference_content


def test_should_not_compress_the_file_before_uploading_to_stage_when_auto_compress_set_to_false(connection):
    """Test that should not compress the file before uploading to stage when AUTO_COMPRESS set to false."""
    uncompressed_file_path = shared_test_data_dir() / "compression" / "test_data.csv"
    uncompressed_filename = "test_data.csv"
    compressed_filename = "test_data.csv.gz"

    # Given Snowflake client is logged in
    with connection.cursor() as cursor:
        stage_name = create_temporary_stage(cursor, "TEST_PUT_GET_AUTO_COMPRESS_FALSE")

        # When File is uploaded to stage with AUTO_COMPRESS set to false
        upload_result = upload_file_to_stage(
            cursor,
            stage_name,
            uncompressed_file_path,
            auto_compress=False,
            overwrite=True
        )

        assert upload_result[6] == "UPLOADED"

        with tempfile.TemporaryDirectory() as temp_dir:
            download_dir = Path(temp_dir)
            
            get_result = get_file_from_stage(cursor, stage_name, uncompressed_filename, download_dir)
            
            assert get_result[2] == "DOWNLOADED"
            
            expected_file_path = download_dir / uncompressed_filename
            assert expected_file_path.exists(), f"Uncompressed file should exist at {expected_file_path}"
            
            # Then Only uncompressed file should be downloaded
            not_expected_file_path = download_dir / compressed_filename
            assert not not_expected_file_path.exists(), f"Compressed file should not exist at {not_expected_file_path}"
            
            # And Have correct content
            downloaded_content = expected_file_path.read_bytes()
            reference_content = uncompressed_file_path.read_bytes()
            assert downloaded_content == reference_content, "Downloaded content should match reference content"