import gzip
import tempfile

from pathlib import Path

from tests.compatibility import NEW_DRIVER_ONLY, OLD_DRIVER_ONLY
from tests.e2e.put_get.put_get_helper import (
    as_file_uri,
    create_temporary_stage,
    create_temporary_stage_and_upload_file,
    get_file_from_stage,
    list_stage_contents,
)
from tests.e2e.types.utils import assert_connection_is_open
from tests.utils import shared_test_data_dir


def test_should_select_data_from_file_uploaded_to_stage(connection):
    test_file_path = shared_test_data_dir() / "compression" / "test_data.csv"

    with connection.cursor() as cursor:
        # Given File is uploaded to stage
        stage_name, _ = create_temporary_stage_and_upload_file(
            cursor,
            "TEST_STAGE_SELECT",
            test_file_path,
            auto_compress=True,
            overwrite=True,
        )

        # When File data is queried using Select command
        select_sql = f"SELECT $1, $2, $3 FROM @{stage_name}"
        cursor.execute(select_sql)

        # Then File data should be correctly returned
        row = cursor.fetchone()
        assert row == ("1", "2", "3")


def test_should_list_file_uploaded_to_stage(connection):
    test_file_path = shared_test_data_dir() / "compression" / "test_data.csv"
    filename = test_file_path.name

    with connection.cursor() as cursor:
        # Given File is uploaded to stage
        stage_name, _ = create_temporary_stage_and_upload_file(
            cursor, "TEST_STAGE_LS", test_file_path, auto_compress=True, overwrite=True
        )

        # When Stage content is listed using LS command
        files = list_stage_contents(cursor, stage_name)

        # Then File should be listed with correct filename
        assert len(files) == 1
        file_info = files[0]
        assert filename + ".gz" in file_info[0]


def test_should_get_file_uploaded_to_stage(connection):
    test_file_path = shared_test_data_dir() / "compression" / "test_data.csv"
    filename = test_file_path.name

    with connection.cursor() as cursor:
        # Given File is uploaded to stage
        stage_name, _ = create_temporary_stage_and_upload_file(
            cursor, "TEST_STAGE_GET", test_file_path, auto_compress=True, overwrite=True
        )
        with tempfile.TemporaryDirectory() as temp_dir:
            # When File is downloaded using GET command
            download_dir = Path(temp_dir)

            get_result = get_file_from_stage(cursor, stage_name, filename, download_dir)

            # Then File should be downloaded
            assert get_result[2] == "DOWNLOADED"
            downloaded_file = download_dir / (filename + ".gz")
            assert downloaded_file.exists()

            # And Have correct content
            with gzip.open(downloaded_file, "rt") as f:
                content = f.read().strip()
                assert content == "1,2,3"


def test_should_return_correct_rowset_for_put(execute_query, connection):
    test_file_path = shared_test_data_dir() / "compression" / "test_data.csv"

    with connection.cursor() as cursor:
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When File is uploaded to stage
        _, upload_result = create_temporary_stage_and_upload_file(
            cursor,
            "TEST_STAGE_PUT_ROWSET",
            test_file_path,
            auto_compress=True,
            overwrite=True,
        )

        # Then Rowset for PUT command should be correct
        assert upload_result[0] == "test_data.csv"
        assert upload_result[1] == "test_data.csv.gz"
        assert upload_result[2] == 6
        if OLD_DRIVER_ONLY("BD#1"):
            assert upload_result[3] == 48
        elif NEW_DRIVER_ONLY("BD#1"):
            assert upload_result[3] == 32
        assert upload_result[4] == "NONE"
        assert upload_result[5] == "GZIP"
        assert upload_result[6] == "UPLOADED"
        assert upload_result[7] == ""


def test_should_return_correct_rowset_for_get(connection):
    test_file_path = shared_test_data_dir() / "compression" / "test_data.csv"
    filename = test_file_path.name

    with connection.cursor() as cursor:
        # Given File is uploaded to stage
        stage_name, _ = create_temporary_stage_and_upload_file(
            cursor,
            "TEST_STAGE_GET_ROWSET",
            test_file_path,
            auto_compress=True,
            overwrite=True,
        )
        with tempfile.TemporaryDirectory() as temp_dir:
            # When File is downloaded using GET command
            download_dir = Path(temp_dir)
            get_result = get_file_from_stage(cursor, stage_name, filename, download_dir)

            # Then Rowset for GET command should be correct
            assert get_result[0] == "test_data.csv.gz"
            if OLD_DRIVER_ONLY("BD#1"):
                assert get_result[1] == 42
            elif NEW_DRIVER_ONLY("BD#1"):
                assert get_result[1] == 26
            assert get_result[2] == "DOWNLOADED"
            assert get_result[3] == ""


def test_should_return_correct_column_metadata_for_put(execute_query, connection):
    test_file_path = shared_test_data_dir() / "compression" / "test_data.csv"

    with connection.cursor() as cursor:
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When File is uploaded to stage
        _, upload_result = create_temporary_stage_and_upload_file(
            cursor,
            "TEST_STAGE_PUT_COLUMN_METADATA",
            test_file_path,
            auto_compress=True,
            overwrite=True,
        )

        # Then Column metadata for PUT command should be correct
        columns = cursor.description
        assert columns is not None, "cursor.description should not be None after PUT"
        assert len(columns) == 8, "PUT command should return 8 columns"
        assert upload_result[6] == "UPLOADED"

        # Verify column names and type codes (TEXT=2, FIXED=0)
        expected_columns = [
            ("source", 2),
            ("target", 2),
            ("source_size", 0),
            ("target_size", 0),
            ("source_compression", 2),
            ("target_compression", 2),
            ("status", 2),
            ("message", 2),
        ]
        for i, (expected_name, expected_type_code) in enumerate(expected_columns):
            actual_name = columns[i][0].lower()
            actual_type_code = columns[i][1]
            assert actual_name == expected_name, f"Column {i} should be named '{expected_name}', got '{actual_name}'"
            assert actual_type_code == expected_type_code, (
                f"Column '{expected_name}' type_code should be {expected_type_code}, got {actual_type_code}"
            )


def test_should_return_correct_column_metadata_for_get(connection):
    test_file_path = shared_test_data_dir() / "compression" / "test_data.csv"
    filename = test_file_path.name

    with connection.cursor() as cursor:
        # Given File is uploaded to stage
        stage_name, _ = create_temporary_stage_and_upload_file(
            cursor,
            "TEST_STAGE_GET_COLUMN_METADATA",
            test_file_path,
            auto_compress=True,
            overwrite=True,
        )

        # When File is downloaded using GET command
        with tempfile.TemporaryDirectory() as temp_dir:
            download_dir = Path(temp_dir)
            get_result = get_file_from_stage(cursor, stage_name, filename, download_dir)

            # Then Column metadata for GET command should be correct
            columns = cursor.description
            assert columns is not None, "cursor.description should not be None after GET"
            assert len(columns) == 4, "GET command should return 4 columns"
            assert get_result[2] == "DOWNLOADED"

            # Verify column names and type codes (TEXT=2, FIXED=0)
            expected_columns = [
                ("file", 2),
                ("size", 0),
                ("status", 2),
                ("message", 2),
            ]
            for i, (expected_name, expected_type_code) in enumerate(expected_columns):
                actual_name = columns[i][0].lower()
                actual_type_code = columns[i][1]
                assert actual_name == expected_name, (
                    f"Column {i} should be named '{expected_name}', got '{actual_name}'"
                )
                assert actual_type_code == expected_type_code, (
                    f"Column '{expected_name}' type_code should be {expected_type_code}, got {actual_type_code}"
                )


def test_should_get_file_from_subdirectory_in_stage(connection):
    test_file_path = shared_test_data_dir() / "compression" / "test_data.csv"
    filename = test_file_path.name

    with connection.cursor() as cursor:
        # Given File is uploaded to a subdirectory in stage
        stage_name = create_temporary_stage(cursor, "TEST_SUBDIR_GET")
        subdir = "nested/subdir"
        file_uri = as_file_uri(test_file_path)
        put_command = f"PUT 'file://{file_uri}' @{stage_name}/{subdir} AUTO_COMPRESS=FALSE OVERWRITE=TRUE"
        cursor.execute(put_command)
        upload_result = cursor.fetchone()
        assert upload_result[6] == "UPLOADED"

        with tempfile.TemporaryDirectory() as temp_dir:
            # When All files are downloaded from stage using GET command
            download_dir = Path(temp_dir)
            download_uri = as_file_uri(download_dir)
            get_command = f"GET @{stage_name}/ 'file://{download_uri}/'"
            cursor.execute(get_command)
            get_result = cursor.fetchone()

            # Then File should be downloaded flat into the local directory
            assert get_result[2] == "DOWNLOADED"
            downloaded_file = download_dir / filename
            assert downloaded_file.exists(), (
                f"Expected file at {downloaded_file}, but directory contents: {list(download_dir.rglob('*'))}"
            )

            # And Have correct content
            content = downloaded_file.read_text().strip()
            assert content == "1,2,3"


def test_should_upload_file_to_subdirectory_in_stage(execute_query, connection):
    test_file_path = shared_test_data_dir() / "compression" / "test_data.csv"
    filename = test_file_path.name

    with connection.cursor() as cursor:
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When File is uploaded to a subdirectory in stage
        stage_name = create_temporary_stage(cursor, "TEST_SUBDIR_UPLOAD")
        subdir_path = f"@{stage_name}/nested/subdir"
        file_uri = as_file_uri(test_file_path)
        put_command = f"PUT 'file://{file_uri}' {subdir_path} AUTO_COMPRESS=FALSE"
        cursor.execute(put_command)
        upload_result = cursor.fetchone()
        assert upload_result[6] == "UPLOADED"

        # Then File should be listed under the subdirectory
        files = list_stage_contents(cursor, stage_name)
        listed_names = [row[0] for row in files]
        assert any("nested/subdir" in name and filename in name for name in listed_names)
