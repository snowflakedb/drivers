import pytest
import tempfile
import gzip
from pathlib import Path

from tests.compatibility import NEW_DRIVER_ONLY, OLD_DRIVER_ONLY
from tests.e2e.put_get.put_get_helper import (
    list_stage_contents,
    get_file_from_stage,
    put_get_test_setup,
)
from tests.utils import shared_test_data_dir


def test_should_select_data_from_file_uploaded_to_stage(connection):
    """Test that should select data from file uploaded to stage."""
    test_file_path = shared_test_data_dir() / "compression" / "test_data.csv"

    # Given File is uploaded to stage
    with put_get_test_setup(
        connection,
        "TEST_STAGE_SELECT",
        test_file_path,
        auto_compress=True,
        overwrite=True,
    ) as (cursor, stage_name, _):
        # When File data is queried using Select command
        select_sql = f"SELECT $1, $2, $3 FROM @{stage_name}"
        cursor.execute(select_sql)

        # Then File data should be correctly returned
        row = cursor.fetchone()
        assert row == ("1", "2", "3")


def test_should_list_file_uploaded_to_stage(connection):
    """Test that should list file uploaded to stage."""
    test_file_path = shared_test_data_dir() / "compression" / "test_data.csv"
    filename = test_file_path.name

    # Given File is uploaded to stage
    with put_get_test_setup(
        connection, "TEST_STAGE_LS", test_file_path, auto_compress=True, overwrite=True
    ) as (cursor, stage_name, _):
        # When Stage content is listed using LS command
        files = list_stage_contents(cursor, stage_name)

        # Then File should be listed with correct filename
        assert len(files) == 1
        file_info = files[0]
        assert filename + ".gz" in file_info[0]


def test_should_get_file_uploaded_to_stage(connection):
    """Test that should get file uploaded to stage."""
    test_file_path = shared_test_data_dir() / "compression" / "test_data.csv"
    filename = test_file_path.name

    # Given File is uploaded to stage
    with put_get_test_setup(
        connection, "TEST_STAGE_GET", test_file_path, auto_compress=True, overwrite=True
    ) as (cursor, stage_name, _):
        # When File is downloaded using GET command
        with tempfile.TemporaryDirectory() as temp_dir:
            download_dir = Path(temp_dir)

            get_result = get_file_from_stage(cursor, stage_name, filename, download_dir)

            # Then File should be downloaded
            assert get_result.status == "DOWNLOADED"
            downloaded_file = download_dir / (filename + ".gz")
            assert downloaded_file.exists()

            # And Have correct content
            with gzip.open(downloaded_file, "rt") as f:
                content = f.read().strip()
                assert content == "1,2,3"


def test_should_return_correct_rowset_for_put(connection):
    """Test that should return correct rowset for PUT."""
    test_file_path = shared_test_data_dir() / "compression" / "test_data.csv"

    # Given Snowflake client is logged in
    # When File is uploaded to stage
    with put_get_test_setup(
        connection,
        "TEST_STAGE_PUT_ROWSET",
        test_file_path,
        auto_compress=True,
        overwrite=True,
    ) as (cursor, stage_name, upload_result):
        # Then Rowset for PUT command should be correct
        assert upload_result.source == "test_data.csv"
        assert upload_result.target == "test_data.csv.gz"
        assert upload_result.source_size == 6
        if OLD_DRIVER_ONLY("BC#1"):
            assert upload_result.target_size == 48
        if NEW_DRIVER_ONLY("BC#1"):
            assert upload_result.target_size == 32
        assert upload_result.source_compression == "NONE"
        assert upload_result.target_compression == "GZIP"
        assert upload_result.status == "UPLOADED"
        assert upload_result.message == ""


def test_should_return_correct_rowset_for_get(connection):
    """Test that should return correct rowset for GET."""
    test_file_path = shared_test_data_dir() / "compression" / "test_data.csv"
    filename = test_file_path.name

    # Given File is uploaded to stage
    with put_get_test_setup(
        connection,
        "TEST_STAGE_GET_ROWSET",
        test_file_path,
        auto_compress=True,
        overwrite=True,
    ) as (cursor, stage_name, _):
        # When File is downloaded using GET command
        with tempfile.TemporaryDirectory() as temp_dir:
            download_dir = Path(temp_dir)

            get_result = get_file_from_stage(cursor, stage_name, filename, download_dir)

            # Then Rowset for GET command should be correct
            assert get_result.file == "test_data.csv.gz"
            if OLD_DRIVER_ONLY("BC#1"):
                assert get_result.size == 42
            if NEW_DRIVER_ONLY("BC#1"):
                assert get_result.size == 26
            assert get_result.status == "DOWNLOADED"
            assert get_result.message == ""


@pytest.mark.skip(reason="cursor.description not implemented in new driver")
def test_should_return_correct_column_metadata_for_put(connection):
    """Test that should return correct column metadata for PUT."""
    test_file_path = shared_test_data_dir() / "compression" / "test_data.csv"

    # Given Snowflake client is logged in
    # When File is uploaded to stage
    with put_get_test_setup(
        connection,
        "TEST_STAGE_PUT_COLUMN_METADATA",
        test_file_path,
        auto_compress=True,
        overwrite=True,
    ) as (cursor, _, upload_result):
        # Then Column metadata for PUT command should be correct
        columns = cursor.description
        assert len(columns) == 8, "PUT command should return 8 columns"
        assert upload_result.status == "UPLOADED"
        expected_columns = [
            "source",
            "target",
            "source_size",
            "target_size",
            "source_compression",
            "target_compression",
            "status",
            "message",
        ]

        for i, expected_name in enumerate(expected_columns):
            actual_name = columns[i][0].lower()
            assert (
                actual_name == expected_name
            ), f"Column {i} should be named '{expected_name}', got '{actual_name}'"


@pytest.mark.skip(reason="cursor.description not implemented in new driver")
def test_should_return_correct_column_metadata_for_get(connection):
    """Test that should return correct column metadata for GET."""
    test_file_path = shared_test_data_dir() / "compression" / "test_data.csv"
    filename = test_file_path.name

    # Given File is uploaded to stage
    with put_get_test_setup(
        connection,
        "TEST_STAGE_GET_COLUMN_METADATA",
        test_file_path,
        auto_compress=True,
        overwrite=True,
    ) as (cursor, stage_name, _):
        # When File is downloaded using GET command
        with tempfile.TemporaryDirectory() as temp_dir:
            download_dir = Path(temp_dir)

            get_result = get_file_from_stage(cursor, stage_name, filename, download_dir)

            # Then Column metadata for GET command should be correct
            columns = cursor.description
            assert len(columns) == 4, "GET command should return 4 columns"
            assert get_result.status == "DOWNLOADED"
            expected_columns = ["file", "size", "status", "message"]
            for i, expected_name in enumerate(expected_columns):
                actual_name = columns[i][0].lower()
                assert (
                    actual_name == expected_name
                ), f"Column {i} should be named '{expected_name}', got '{actual_name}'"
