import pytest
import tempfile
import gzip
from pathlib import Path

from tests.compatibility import NEW_DRIVER_ONLY, OLD_DRIVER_ONLY
from tests.e2e.put_get.put_get_helper import create_temporary_stage, upload_file_to_stage, list_stage_contents, get_file_from_stage
from tests.utils import shared_test_data_dir


def test_should_select_data_from_file_uploaded_to_stage(connection):
    """Test that should select data from file uploaded to stage."""
    test_file_path = shared_test_data_dir() / "compression" / "test_data.csv"

    # Given File is uploaded to stage
    with connection.cursor() as cursor:
        # Create temporary stage with unique name
        stage_name = create_temporary_stage(cursor, "TEST_STAGE_SELECT")

        # Upload file to stage using helper method
        upload_result = upload_file_to_stage(
            cursor,
            stage_name,
            test_file_path,
            auto_compress=True,
            overwrite=True
        )
        
        assert upload_result[6] == "UPLOADED"  # status column
        
        # When File data is queried using Select command
        select_sql = f"SELECT $1, $2, $3 FROM @{stage_name}"
        cursor.execute(select_sql)

        # Then File data should be correctly returned
        row = cursor.fetchone()
        assert row == ("1", "2", "3")


def test_should_list_file_uploaded_to_stage(connection):
    """Test that should list file uploaded to stage."""
    test_file_path = shared_test_data_dir() / "compression" / "test_data.csv"
    filename = test_file_path.name  # "test_data.csv"
    
    # Given File is uploaded to stage
    with connection.cursor() as cursor:
        stage_name = create_temporary_stage(cursor, "TEST_STAGE_LS")
        
        # Upload file to stage using helper method
        upload_result = upload_file_to_stage(
            cursor,
            stage_name,
            test_file_path,
            auto_compress=True,
            overwrite=True
        )
        
        assert upload_result[6] == "UPLOADED"  # status column
        
        # When Stage content is listed using LS command
        files = list_stage_contents(cursor, stage_name)
        
        # Then File should be listed with correct filename
        assert len(files) == 1
        file_info = files[0]
        # File should be compressed (test_data.csv.gz)
        assert filename + ".gz" in file_info[0]  # file name


def test_should_get_file_uploaded_to_stage(connection):
    """Test that should get file uploaded to stage."""
    test_file_path = shared_test_data_dir() / "compression" / "test_data.csv"
    filename = test_file_path.name  # "test_data.csv"
    
    # Given File is uploaded to stage
    with connection.cursor() as cursor:
        stage_name = create_temporary_stage(cursor, "TEST_STAGE_GET")
        
        upload_result = upload_file_to_stage(
            cursor,
            stage_name,
            test_file_path,
            auto_compress=True,
            overwrite=True
        )
        
        assert upload_result[6] == "UPLOADED"  # status column
        
        # When File is downloaded using GET command
        with tempfile.TemporaryDirectory() as temp_dir:
            download_dir = Path(temp_dir)
            
            get_result = get_file_from_stage(cursor, stage_name, filename, download_dir)
            
            # Then File should be downloaded
            assert get_result[2] == "DOWNLOADED"  # status
            downloaded_file = download_dir / (filename + ".gz")
            assert downloaded_file.exists()
            
            # And Have correct content
            with gzip.open(downloaded_file, 'rt') as f:
                content = f.read().strip()
                assert content == "1,2,3"


def test_should_return_correct_rowset_for_put(connection):
    """Test that should return correct rowset for PUT."""
    test_file_path = shared_test_data_dir() / "compression" / "test_data.csv"
    
    # Given Snowflake client is logged in
    with connection.cursor() as cursor:
        stage_name = create_temporary_stage(cursor, "TEST_STAGE_PUT_ROWSET")
        
        # When File is uploaded to stage
        upload_result = upload_file_to_stage(
            cursor,
            stage_name,
            test_file_path,
            auto_compress=True,
            overwrite=True
        )
        
        # Then Rowset for PUT command should be correct
        assert upload_result[0] == "test_data.csv"  # source_file
        assert upload_result[1] == "test_data.csv.gz"  # target_file
        assert upload_result[2] == 6  # source_size
        if OLD_DRIVER_ONLY("BC#1"):
            assert upload_result[3] == 48  # target_size
        if NEW_DRIVER_ONLY("BC#1"):
            assert upload_result[3] == 32  # target_size
        assert upload_result[4] == "NONE"  # source_compression
        assert upload_result[5] == "GZIP"  # target_compression
        assert upload_result[6] == "UPLOADED"  # status
        assert upload_result[7] == ""  # message


def test_should_return_correct_rowset_for_get(connection):
    """Test that should return correct rowset for GET."""
    test_file_path = shared_test_data_dir() / "compression" / "test_data.csv"
    filename = test_file_path.name  # "test_data.csv"
    
    # Given File is uploaded to stage
    with connection.cursor() as cursor:
        stage_name = create_temporary_stage(cursor, "TEST_STAGE_GET_ROWSET")
        
        upload_result = upload_file_to_stage(
            cursor,
            stage_name,
            test_file_path,
            auto_compress=True,
            overwrite=True
        )
        
        assert upload_result[6] == "UPLOADED"  # status column
        
        # When File is downloaded using GET command
        with tempfile.TemporaryDirectory() as temp_dir:
            download_dir = Path(temp_dir)
            
            get_result = get_file_from_stage(cursor, stage_name, filename, download_dir)
            
            # Then Rowset for GET command should be correct
            assert get_result[0] == "test_data.csv.gz"  # file
            if OLD_DRIVER_ONLY("BC#1"):
                assert get_result[1] == 42  # size
            if NEW_DRIVER_ONLY("BC#1"):
                assert get_result[1] == 26  # size
            assert get_result[2] == "DOWNLOADED"  # status
            assert get_result[3] == ""  # message


@pytest.mark.skip(reason="cursor.description not implemented in new driver")
def test_should_return_correct_column_metadata_for_put(connection):
    """Test that should return correct column metadata for PUT."""
    test_file_path = shared_test_data_dir() / "compression" / "test_data.csv"
    
    # Given Snowflake client is logged in
    with connection.cursor() as cursor:
        stage_name = create_temporary_stage(cursor, "TEST_STAGE_PUT_COLUMN_METADATA")
        
        # When File is uploaded to stage
        upload_result = upload_file_to_stage(
            cursor,
            stage_name,
            test_file_path,
            auto_compress=True,
            overwrite=True
        )
        
        # Then Column metadata for PUT command should be correct
        columns = cursor.description
        assert len(columns) == 8, "PUT command should return 8 columns"
        
        # Verify upload was successful
        assert upload_result[6] == "UPLOADED"  # status
        
        # Verify column names and types
        expected_columns = [
            "source",
            "target", 
            "source_size",
            "target_size",
            "source_compression",
            "target_compression",
            "status",
            "message"
        ]
        
        for i, expected_name in enumerate(expected_columns):
            actual_name = columns[i][0].lower()
            assert actual_name == expected_name, f"Column {i} should be named '{expected_name}', got '{actual_name}'"


@pytest.mark.skip(reason="cursor.description not implemented in new driver")
def test_should_return_correct_column_metadata_for_get(connection):
    """Test that should return correct column metadata for GET."""
    test_file_path = shared_test_data_dir() / "compression" / "test_data.csv"
    filename = test_file_path.name  # "test_data.csv"
    
    # Given File is uploaded to stage
    with connection.cursor() as cursor:
        stage_name = create_temporary_stage(cursor, "TEST_STAGE_GET_COLUMN_METADATA")
        
        upload_result = upload_file_to_stage(
            cursor,
            stage_name,
            test_file_path,
            auto_compress=True,
            overwrite=True
        )
        
        assert upload_result[6] == "UPLOADED"  # status column
        
        # When File is downloaded using GET command
        with tempfile.TemporaryDirectory() as temp_dir:
            download_dir = Path(temp_dir)
            
            get_result = get_file_from_stage(cursor, stage_name, filename, download_dir)
            
            # Then Column metadata for GET command should be correct
            columns = cursor.description
            assert len(columns) == 4, "GET command should return 4 columns"
            
            # Verify download was successful
            assert get_result[2] == "DOWNLOADED"
            
            # Verify column names
            expected_columns = [
                "file",
                "size",
                "status", 
                "message"
            ]
            
            for i, expected_name in enumerate(expected_columns):
                actual_name = columns[i][0].lower()  # Column name is first element
                assert actual_name == expected_name, f"Column {i} should be named '{expected_name}', got '{actual_name}'"
